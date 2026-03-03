//! Build script for nzea simulator: 1) nzea just run generates Verilog; 2) verilator --build compiles RTL;
//! 3) compile nzea_wrapper.cpp and link with Verilator's libVTop.a, libverilated.a.
//! Verilator headers use -isystem to suppress internal warnings.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

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

/// Verilator include path (for verilated.h in VTop.h). Prefer VERILATOR_ROOT, else derive from which verilator.
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

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut linked_real = false;

    let verilog_out = out_dir.join("nzea-verilog");
    std::fs::create_dir_all(&verilog_out).expect("create nzea-verilog");
    let verilog_out_abs = verilog_out.canonicalize().expect("canonicalize nzea-verilog");

    let workspace_root = find_workspace_root(&manifest_dir);
    let nzea_dir: PathBuf = env::var("NZEA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root.join("..").join("nzea"));
    let nzea_dir_abs = nzea_dir.canonicalize().unwrap_or_else(|_| nzea_dir.clone());
    let justfile = nzea_dir_abs.join("justfile");

    if !justfile.exists() {
        panic!(
            "nzea build failed: justfile not found: {} (set NZEA_DIR if needed)",
            justfile.display()
        );
    }

    let status = Command::new("direnv")
        .arg("exec")
        .arg(&nzea_dir_abs)
        .arg("just")
        .arg("--justfile")
        .arg(&justfile)
        .arg("run")
        .arg("--outDir")
        .arg(&verilog_out_abs)
        .current_dir(&workspace_root)
        .status();

    let just_ok = match status {
        Ok(s) if s.success() => true,
        Ok(s) => {
            eprintln!("cargo:warning=nzea just run failed: {:?}", s.code());
            false
        }
        Err(e) => {
            eprintln!("cargo:warning=nzea Verilog generation failed: {}", e);
            false
        }
    };

    let top_sv = verilog_out_abs.join("Top.sv");
    if just_ok && top_sv.exists() {
        let v_build = out_dir.join("verilator_build");
        std::fs::create_dir_all(&v_build).expect("create verilator_build");

        let filelist_path = verilog_out.join("filelist.f");
        let sv_files: Vec<String> = if filelist_path.exists() {
            std::fs::read_to_string(&filelist_path)
                .unwrap_or_default()
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| s.ends_with(".sv"))
                .map(|s| format!("nzea-verilog/{}", s))
                .collect()
        } else {
            vec!["nzea-verilog/Top.sv".to_string()]
        };

        let out_dir_escaped = out_dir.as_os_str().to_string_lossy().replace('\'', "'\"'\"'");
        let files_arg = sv_files.join(" ");
        let verilator_cmd = format!(
            "cd '{}' && verilator --cc --build --trace-fst --Mdir verilator_build --top-module Top {}",
            out_dir_escaped, files_arg
        );

        let verilator_out = Command::new("sh").args(["-c", &verilator_cmd]).output();
        let verilator_ok = match &verilator_out {
            Ok(out) if out.status.success() => true,
            Ok(out) => {
                eprintln!("cargo:warning=verilator stderr: {}", String::from_utf8_lossy(&out.stderr));
                eprintln!("cargo:warning=verilator stdout: {}", String::from_utf8_lossy(&out.stdout));
                false
            }
            Err(e) => {
                eprintln!("cargo:warning=verilator spawn failed: {}", e);
                false
            }
        };

        if verilator_ok {
            let lib_vtop = v_build.join("libVTop.a");
            let lib_verilated = v_build.join("libverilated.a");
            if lib_vtop.exists() && lib_verilated.exists() {
                let v_include = match find_verilator_include() {
                    Some(p) => p,
                    None => {
                        eprintln!("cargo:warning=Verilator include not found, set VERILATOR_ROOT");
                        return;
                    }
                };
                let v_include_vltstd = v_include.join("vltstd");
                let wrapper_src = manifest_dir.join("c_src").join("nzea_wrapper.cpp");
                let opt = match env::var("PROFILE").as_deref() {
                    Ok("release") => 3,
                    _ => 1, // -O1 for debug to avoid _FORTIFY_SOURCE etc.
                };

                let mut build = cc::Build::new();
                build.cpp(true).std("c++17").opt_level(opt).include(&v_build);
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
                    build.include(&v_include).include(&v_include_vltstd);
                }
                build.file(&wrapper_src).compile("nzea_wrapper");

                println!("cargo:rustc-link-search=native={}", v_build.display());
                println!("cargo:rustc-link-lib=static=VTop");
                println!("cargo:rustc-link-lib=static=verilated");
                println!("cargo:rustc-link-lib=z"); // FST waveform needs zlib
                #[cfg(not(target_env = "msvc"))]
                {
                    if cfg!(target_os = "macos") {
                        println!("cargo:rustc-link-lib=dylib=c++");
                    } else {
                        println!("cargo:rustc-link-lib=dylib=stdc++");
                    }
                }
                linked_real = true;
            } else {
                eprintln!("cargo:warning=verilator --build did not produce libVTop.a / libverilated.a");
            }
        }
    }

    if !linked_real {
        panic!(
            "nzea build failed. Ensure: 1) nzea RTL is generated (just run); 2) verilator is in PATH"
        );
    }

    // Verilator may create files (e.g. verilator_include symlinks) with read-only permissions,
    // causing "Permission denied" when cargo clean tries to remove them. Fix before exit.
    #[cfg(unix)]
    {
        let _ = Command::new("chmod").args(["-R", "u+w"]).arg(&out_dir).status();
    }

    println!("cargo:rerun-if-env-changed=NZEA_DIR");
    println!("cargo:rerun-if-env-changed=VERILATOR_ROOT");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=c_src/nzea_wrapper.cpp");
    // Watch justfile and src/; avoid watching whole nzea (just run writes to target/ etc. and triggers rebuilds)
    println!("cargo:rerun-if-changed={}", justfile.display());
    let nzea_src = nzea_dir_abs.join("src");
    if nzea_src.is_dir() {
        println!("cargo:rerun-if-changed={}", nzea_src.display());
    }
}
