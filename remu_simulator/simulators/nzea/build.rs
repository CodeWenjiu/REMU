//! Build script for nzea simulator.
//!
//! Pipeline: 1) just dump --isa <isa> for each ISA (Chisel → Verilog in subdirs);
//! 2) verilator --build for each ISA in parallel (with --prefix to avoid symbol conflicts);
//! 3) compile nzea_wrapper.cpp and link with all libVTop_<isa>.a, libverilated.a.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::thread;

const NZEA_ISAS: &[&str] = &["riscv32i", "riscv32im"];

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

/// Run `just dump --isa <isa> --outDir <verilog_out>` for one ISA.
fn run_verilog_generation_for_isa(
    nzea_dir: &Path,
    verilog_out: &Path,
    workspace_root: &Path,
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
        .arg("exec")
        .arg(nzea_dir)
        .arg("just")
        .arg("--justfile")
        .arg(&justfile)
        .arg("dump")
        .arg("--isa")
        .arg(isa)
        .arg("--outDir")
        .arg(verilog_out)
        .current_dir(workspace_root)
        .status();

    match status {
        Ok(s) if s.success() => true,
        Ok(s) => {
            eprintln!("cargo:warning=nzea just dump --isa {} failed: {:?}", isa, s.code());
            false
        }
        Err(e) => {
            eprintln!("cargo:warning=nzea Verilog generation for {} failed: {}", isa, e);
            false
        }
    }
}

/// Collect .sv file paths from filelist.f, or fallback to Top.sv.
fn collect_sv_files(verilog_dir: &Path, prefix: &str) -> Vec<String> {
    let filelist_path = verilog_dir.join("filelist.f");
    if filelist_path.exists() {
        let files: Vec<String> = std::fs::read_to_string(&filelist_path)
            .unwrap_or_default()
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| s.ends_with(".sv"))
            .map(|s| format!("{}/{}", prefix, s))
            .collect();
        if !files.is_empty() {
            return files;
        }
    }
    vec![format!("{}/Top.sv", prefix)]
}

/// Run verilator --cc --build for one ISA.
fn run_verilator_for_isa(
    out_dir: &Path,
    verilog_dir: &Path,
    isa: &str,
    prefix: &str,
) -> bool {
    let v_build = out_dir.join(format!("verilator_build_{}", isa));
    std::fs::create_dir_all(&v_build).expect("create verilator_build dir");

    let cc = env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let cxx = env::var("CXX").unwrap_or_else(|_| "g++".to_string());
    let ccache_cc = format!("ccache {}", cc);
    let ccache_cxx = format!("ccache {}", cxx);

    let out_dir_escaped = out_dir.as_os_str().to_string_lossy().replace('\'', "'\"'\"'");
    let sv_files = collect_sv_files(verilog_dir, &format!("nzea-verilog/{}", isa));
    let files_arg = sv_files.join(" ");
    let makeflags = format!("CC=\"{}\" CXX=\"{}\"", ccache_cc, ccache_cxx);
    let cmd = format!(
        "cd '{}' && verilator --cc --build --trace-fst -MAKEFLAGS '{}' --Mdir {} --top-module Top --prefix {} {}",
        out_dir_escaped, makeflags, v_build.display(), prefix, files_arg
    );

    let mut cmd_build = Command::new("sh");
    cmd_build.args(["-c", &cmd]);
    cmd_build.env("CC", &ccache_cc);
    cmd_build.env("CXX", &ccache_cxx);
    match cmd_build.output() {
        Ok(out) if out.status.success() => true,
        Ok(out) => {
            eprintln!(
                "cargo:warning=verilator for {} stderr: {}",
                isa,
                String::from_utf8_lossy(&out.stderr)
            );
            eprintln!(
                "cargo:warning=verilator for {} stdout: {}",
                isa,
                String::from_utf8_lossy(&out.stdout)
            );
            false
        }
        Err(e) => {
            eprintln!("cargo:warning=verilator for {} spawn failed: {}", isa, e);
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

/// Compile nzea_wrapper.cpp and emit link flags for all ISAs.
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

    for isa in NZEA_ISAS {
        let v_build = out_dir.join(format!("verilator_build_{}", isa));
        build.include(&v_build);
    }

    #[cfg(not(target_env = "msvc"))]
    {
        build.flag("-isystem");
        build.flag(v_include.to_string_lossy().as_ref());
        build.flag("-isystem");
        build.flag(v_include_vltstd.to_string_lossy().as_ref());
        build.flag("-Wno-unused-parameter").flag("-Wno-sign-compare");
    }
    #[cfg(target_env = "msvc")]
    {
        build.include(v_include).include(&v_include_vltstd);
    }
    build.file(&wrapper_src).compile("nzea_wrapper");

    for isa in NZEA_ISAS {
        let v_build = out_dir.join(format!("verilator_build_{}", isa));
        println!("cargo:rustc-link-search=native={}", v_build.display());
        let prefix = format!("VTop_{}", isa);
        println!("cargo:rustc-link-lib=static={}", prefix);
    }
    let v_build_first = out_dir.join(format!("verilator_build_{}", NZEA_ISAS[0]));
    println!("cargo:rustc-link-search=native={}", v_build_first.display());
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
    let verilog_out_abs = verilog_out.canonicalize().expect("canonicalize nzea-verilog");

    // Step 1: Generate Verilog for each ISA
    for isa in NZEA_ISAS {
        let isa_out = verilog_out_abs.join(isa);
        std::fs::create_dir_all(&isa_out).expect("create isa verilog dir");
        if !run_verilog_generation_for_isa(&nzea_dir, &isa_out, &workspace_root, isa) {
            panic!("nzea build failed: Verilog generation for {} failed", isa);
        }
        let top_sv = isa_out.join("Top.sv");
        if !top_sv.exists() {
            panic!("nzea build failed: Top.sv not found for {}", isa);
        }
    }

    // Step 2: Run Verilator for each ISA in parallel
    let (tx, rx) = mpsc::channel();
    for isa in NZEA_ISAS {
        let tx = tx.clone();
        let out_dir = out_dir.clone();
        let verilog_dir = verilog_out_abs.join(isa);
        let isa = isa.to_string();
        let prefix = format!("VTop_{}", isa);
        thread::spawn(move || {
            let ok = run_verilator_for_isa(&out_dir, &verilog_dir, &isa, &prefix);
            tx.send((isa, ok)).unwrap();
        });
    }
    drop(tx);
    for (isa, ok) in rx {
        if !ok {
            panic!("nzea build failed: verilator for {} failed", isa);
        }
    }

    // Step 3: Verify libs exist
    for isa in NZEA_ISAS {
        let v_build = out_dir.join(format!("verilator_build_{}", isa));
        let lib = v_build.join(format!("libVTop_{}.a", isa));
        if !lib.exists() {
            panic!("nzea build failed: {} not found", lib.display());
        }
    }
    let v_build_first = out_dir.join(format!("verilator_build_{}", NZEA_ISAS[0]));
    let lib_verilated = v_build_first.join("libverilated.a");
    if !lib_verilated.exists() {
        panic!("nzea build failed: libverilated.a not found");
    }

    // Step 4: Compile wrapper and link
    let v_include = find_verilator_include().unwrap_or_else(|| {
        panic!("nzea build failed: Verilator include not found (set VERILATOR_ROOT)");
    });
    compile_wrapper_and_link(&manifest_dir, &out_dir, &v_include)
        .expect("nzea build failed: wrapper compile");

    #[cfg(unix)]
    {
        let _ = Command::new("chmod").args(["-R", "u+w"]).arg(&out_dir).status();
    }
}
