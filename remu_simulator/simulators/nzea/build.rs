//! Build script for nzea simulator.
//!
//! Pipeline:
//! 1) `just dump --target <core|tile> --isa <isa>` for each (target, isa);
//! 2) `verilator --build` for each model in parallel (with unique --prefix);
//! 3) compile `nzea_wrapper.cpp` and link all generated `libVTop_*` + `libverilated.a`.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::thread;

const NZEA_ISAS: &[&str] = &[
    "riscv32i",
    "riscv32im",
    "riscv32i_wjCus0",
    "riscv32im_wjCus0",
];

/// (target, top_module)
const NZEA_TARGETS: &[(&str, &str)] = &[("core", "Top"), ("tile", "NzeaTile")];

#[inline]
fn model_desc(target: &str, isa: &str) -> String {
    format!("{target}:{isa}")
}

#[inline]
fn model_prefix(target: &str, isa: &str) -> String {
    format!("VTop_{target}_{isa}")
}

#[inline]
fn model_build_dir(out_dir: &Path, target: &str, isa: &str) -> PathBuf {
    out_dir.join(format!("verilator_build_{target}_{isa}"))
}

fn find_workspace_root(manifest_dir: &Path) -> PathBuf {
    for p in manifest_dir.ancestors() {
        let toml = p.join("Cargo.toml");
        if toml.exists() {
            if let Ok(s) = std::fs::read_to_string(&toml) {
                if s.contains("[workspace]") {
                    return p.to_path_buf();
                }
            }
        }
    }
    manifest_dir.to_path_buf()
}

fn resolve_nzea_dir(workspace_root: &Path) -> PathBuf {
    let nzea_dir = env::var("NZEA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root.join("..").join("nzea"));
    nzea_dir.canonicalize().unwrap_or(nzea_dir)
}

/// Recursively collect all .scala files under dir for cargo:rerun-if-changed.
fn collect_scala_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(collect_scala_files(&path));
        } else if path.extension().map(|e| e == "scala").unwrap_or(false) {
            out.push(path);
        }
    }
    out
}

/// Run `just dump --target <target> --isa <isa> --outDir <verilog_out>` for one model.
fn run_verilog_generation_for_model(
    nzea_dir: &Path,
    verilog_out: &Path,
    workspace_root: &Path,
    target: &str,
    isa: &str,
) -> bool {
    let justfile = nzea_dir.join("justfile");
    if !justfile.exists() {
        panic!(
            "nzea build failed: justfile not found: {} (set NZEA_DIR if needed)",
            justfile.display()
        );
    }

    let status = Command::new("direnv")
        .env("REQUIRE_FLAKE", "1")
        .arg("exec")
        .arg(nzea_dir)
        .arg("just")
        .arg("--justfile")
        .arg(&justfile)
        .arg("dump")
        .arg("--target")
        .arg(target)
        .arg("--isa")
        .arg(isa)
        .arg("--outDir")
        .arg(verilog_out)
        .current_dir(workspace_root)
        .status();

    let model = model_desc(target, isa);
    match status {
        Ok(s) if s.success() => true,
        Ok(s) => {
            eprintln!(
                "cargo:warning=nzea just dump for {} failed: {:?}",
                model,
                s.code()
            );
            false
        }
        Err(e) => {
            eprintln!(
                "cargo:warning=nzea Verilog generation for {} failed: {}",
                model, e
            );
            false
        }
    }
}

/// CIRCT firtool emits `<mem>_init.sv` (`bind` + `$readmemb` for `loadMemoryFromFile`) next to
/// `filelist.f` but **does not list** those files in `filelist.f`. Verilator only sees sources on
/// its command line, so omitting them leaves `Memory` arrays zero — NNU weights never load.
fn append_firtool_memory_init_sv(verilog_dir: &Path, prefix: &str, files: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(verilog_dir) else {
        return;
    };
    let mut extra: Vec<String> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name();
            let name = name.to_str()?;
            if !name.ends_with("_init.sv") {
                return None;
            }
            let rel = format!("{prefix}/{name}");
            if files.contains(&rel) {
                return None;
            }
            Some(rel)
        })
        .collect();
    extra.sort();
    files.extend(extra);
}

/// Collect .sv file paths from filelist.f, or fallback to top module .sv file.
fn collect_sv_files(verilog_dir: &Path, prefix: &str, top_module: &str) -> Vec<String> {
    let filelist_path = verilog_dir.join("filelist.f");
    let mut files = if filelist_path.exists() {
        let listed: Vec<String> = std::fs::read_to_string(&filelist_path)
            .unwrap_or_default()
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| s.ends_with(".sv"))
            .map(|s| format!("{prefix}/{s}"))
            .collect();
        if !listed.is_empty() {
            listed
        } else {
            vec![format!("{prefix}/{top_module}.sv")]
        }
    } else {
        vec![format!("{prefix}/{top_module}.sv")]
    };
    append_firtool_memory_init_sv(verilog_dir, prefix, &mut files);
    files
}

/// Run verilator --cc --build for one (target, isa) model.
fn run_verilator_for_model(
    out_dir: &Path,
    verilog_dir: &Path,
    target: &str,
    isa: &str,
    top_module: &str,
) -> bool {
    let v_build = model_build_dir(out_dir, target, isa);
    std::fs::create_dir_all(&v_build).expect("create verilator_build dir");

    let cc = env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let cxx = env::var("CXX").unwrap_or_else(|_| "g++".to_string());
    let ccache_cc = format!("ccache {cc}");
    let ccache_cxx = format!("ccache {cxx}");

    let out_dir_escaped = out_dir
        .as_os_str()
        .to_string_lossy()
        .replace('\'', "'\"'\"'");
    let sv_files = collect_sv_files(
        verilog_dir,
        &format!("nzea-verilog/{target}/{isa}"),
        top_module,
    );
    let files_arg = sv_files.join(" ");
    let makeflags = format!("CC=\"{ccache_cc}\" CXX=\"{ccache_cxx}\"");
    let prefix = model_prefix(target, isa);
    let cmd = format!(
        "cd '{out_dir_escaped}' && verilator --cc --build --trace-fst -MAKEFLAGS '{makeflags}' --Mdir {} --top-module {top_module} --prefix {prefix} {files_arg}",
        v_build.display()
    );

    let mut cmd_build = Command::new("sh");
    cmd_build.args(["-c", &cmd]);
    cmd_build.env("CC", &ccache_cc);
    cmd_build.env("CXX", &ccache_cxx);
    let model = model_desc(target, isa);
    match cmd_build.output() {
        Ok(out) if out.status.success() => true,
        Ok(out) => {
            eprintln!(
                "cargo:warning=verilator for {model} stderr: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            eprintln!(
                "cargo:warning=verilator for {model} stdout: {}",
                String::from_utf8_lossy(&out.stdout)
            );
            false
        }
        Err(e) => {
            eprintln!("cargo:warning=verilator for {model} spawn failed: {e}");
            false
        }
    }
}

fn find_verilator_include() -> Option<PathBuf> {
    if let Ok(root) = env::var("VERILATOR_ROOT") {
        let inc = PathBuf::from(&root).join("include");
        if inc.exists() {
            return Some(inc);
        }
    }
    let path_env = env::var_os("PATH")?;
    let exe_path = env::split_paths(&path_env)
        .map(|p| p.join("verilator"))
        .find(|p| p.is_file())?;
    let exe_canon = std::fs::canonicalize(&exe_path).ok()?;
    let prefix = exe_canon.parent()?.parent()?;
    let candidates = [
        prefix.join("share").join("verilator").join("include"),
        prefix.join("include"),
        PathBuf::from("/usr/share/verilator/include"),
        PathBuf::from("/usr/local/share/verilator/include"),
    ];
    candidates.iter().find(|p| p.exists()).cloned()
}

/// Compile nzea_wrapper.cpp and emit link flags for all generated models.
fn compile_wrapper_and_link(
    manifest_dir: &Path,
    out_dir: &Path,
    v_include: &Path,
) -> Result<(), String> {
    let v_include_vltstd = v_include.join("vltstd");
    let wrapper_src = manifest_dir.join("c_src").join("nzea_wrapper.cpp");
    let opt = match env::var("PROFILE").as_deref() {
        Ok("release") => 3,
        _ => 1,
    };

    let mut build = cc::Build::new();
    build.cpp(true).std("c++17").opt_level(opt);

    for (target, _) in NZEA_TARGETS {
        for isa in NZEA_ISAS {
            build.include(model_build_dir(out_dir, target, isa));
        }
    }

    #[cfg(not(target_env = "msvc"))]
    {
        build.flag("-isystem");
        build.flag(v_include.to_string_lossy().as_ref());
        build.flag("-isystem");
        build.flag(v_include_vltstd.to_string_lossy().as_ref());
        build
            .flag("-Wno-unused-parameter")
            .flag("-Wno-sign-compare");
    }
    #[cfg(target_env = "msvc")]
    {
        build.include(v_include).include(&v_include_vltstd);
    }
    build.file(&wrapper_src).compile("nzea_wrapper");

    for (target, _) in NZEA_TARGETS {
        for isa in NZEA_ISAS {
            let v_build = model_build_dir(out_dir, target, isa);
            println!("cargo:rustc-link-search=native={}", v_build.display());
            println!("cargo:rustc-link-lib=static={}", model_prefix(target, isa));
        }
    }

    // libverilated.a from any generated model build dir is sufficient.
    let first_build = model_build_dir(out_dir, NZEA_TARGETS[0].0, NZEA_ISAS[0]);
    println!("cargo:rustc-link-search=native={}", first_build.display());
    println!("cargo:rustc-link-lib=static=verilated");
    println!("cargo:rustc-link-lib=z");
    #[cfg(not(target_env = "msvc"))]
    {
        if cfg!(target_os = "macos") {
            println!("cargo:rustc-link-lib=dylib=c++");
        } else {
            println!("cargo:rustc-link-lib=dylib=stdc++");
        }
    }
    Ok(())
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let workspace_root = find_workspace_root(&manifest_dir);
    let nzea_dir = resolve_nzea_dir(&workspace_root);

    for path in collect_scala_files(&nzea_dir) {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    let verilog_out = out_dir.join("nzea-verilog");
    std::fs::create_dir_all(&verilog_out).expect("create nzea-verilog");
    let verilog_out_abs = verilog_out
        .canonicalize()
        .expect("canonicalize nzea-verilog");

    // Step 1: Generate Verilog for each (target, isa).
    for (target, top_module) in NZEA_TARGETS {
        for isa in NZEA_ISAS {
            let model = model_desc(target, isa);
            let isa_out = verilog_out_abs.join(target).join(isa);
            std::fs::create_dir_all(&isa_out).expect("create model verilog dir");
            if !run_verilog_generation_for_model(&nzea_dir, &isa_out, &workspace_root, target, isa)
            {
                panic!("nzea build failed: Verilog generation for {} failed", model);
            }
            let top_sv = isa_out.join(format!("{top_module}.sv"));
            let filelist = isa_out.join("filelist.f");
            if !top_sv.exists() && !filelist.exists() {
                panic!(
                    "nzea build failed: neither {} nor filelist.f found for {}",
                    top_sv.display(),
                    model
                );
            }
        }
    }

    // Step 2: Run Verilator for each model in parallel.
    let (tx, rx) = mpsc::channel();
    for (target, top_module) in NZEA_TARGETS {
        for isa in NZEA_ISAS {
            let tx = tx.clone();
            let out_dir = out_dir.clone();
            let verilog_dir = verilog_out_abs.join(target).join(isa);
            let target = (*target).to_string();
            let isa = (*isa).to_string();
            let top_module = (*top_module).to_string();
            thread::spawn(move || {
                let ok =
                    run_verilator_for_model(&out_dir, &verilog_dir, &target, &isa, &top_module);
                tx.send((target, isa, ok)).unwrap();
            });
        }
    }
    drop(tx);
    for (target, isa, ok) in rx {
        if !ok {
            panic!(
                "nzea build failed: verilator for {} failed",
                model_desc(&target, &isa)
            );
        }
    }

    // Step 3: Verify libs exist.
    for (target, _) in NZEA_TARGETS {
        for isa in NZEA_ISAS {
            let model = model_desc(target, isa);
            let v_build = model_build_dir(&out_dir, target, isa);
            let lib = v_build.join(format!("lib{}.a", model_prefix(target, isa)));
            if !lib.exists() {
                panic!(
                    "nzea build failed: {} not found for {}",
                    lib.display(),
                    model
                );
            }
        }
    }

    let v_build_first = model_build_dir(&out_dir, NZEA_TARGETS[0].0, NZEA_ISAS[0]);
    let lib_verilated = v_build_first.join("libverilated.a");
    if !lib_verilated.exists() {
        panic!(
            "nzea build failed: libverilated.a not found in {}",
            v_build_first.display()
        );
    }

    // Step 4: Compile wrapper and link.
    let v_include = find_verilator_include().unwrap_or_else(|| {
        panic!("nzea build failed: Verilator include not found (set VERILATOR_ROOT)");
    });
    compile_wrapper_and_link(&manifest_dir, &out_dir, &v_include)
        .expect("nzea build failed: wrapper compile");

    #[cfg(unix)]
    {
        let _ = Command::new("chmod")
            .args(["-R", "u+w"])
            .arg(&out_dir)
            .status();
    }
}
