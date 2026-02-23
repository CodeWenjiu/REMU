//! Build script for nzea simulator: run the external nzea project's justfile to generate
//! Verilog into target/nzea-verilog (or OUT_DIR/nzea-verilog). Verilator and linking later.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Find workspace root by walking up from manifest dir and looking for Cargo.toml with [workspace].
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

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Verilog output: under OUT_DIR so it lives in target/.../build/.../out/nzea-verilog
    let verilog_out = out_dir.join("nzea-verilog");
    std::fs::create_dir_all(&verilog_out).expect("create nzea-verilog output dir");
    let verilog_out_abs = verilog_out
        .canonicalize()
        .expect("nzea-verilog dir must be canonicalizable");

    // Nzea project root: env NZEA_DIR, or sibling of workspace (remu) root
    let workspace_root = find_workspace_root(&manifest_dir);
    let nzea_dir: PathBuf = env::var("NZEA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root.join("..").join("nzea"));
    let nzea_dir_abs = nzea_dir
        .canonicalize()
        .unwrap_or_else(|_| nzea_dir.clone());
    let justfile = nzea_dir_abs.join("justfile");

    if !justfile.exists() {
        eprintln!(
            "cargo:warning=nzea justfile not found at {}, skipping Verilog generation (set NZEA_DIR if nzea is elsewhere)",
            justfile.display()
        );
        return;
    }

    // Same invocation as from remu root: direnv exec <nzea> just --justfile <nzea>/justfile run --outDir <abs>
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

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            eprintln!(
                "cargo:warning=nzea just run failed with exit code {:?}",
                s.code()
            );
        }
        Err(e) => {
            eprintln!(
                "cargo:warning=nzea Verilog generation failed: {} (is direnv in PATH?)",
                e
            );
        }
    }

    // Rerun when env or build script changes (cannot rerun-if-changed on external nzea paths)
    println!("cargo:rerun-if-env-changed=NZEA_DIR");
    println!("cargo:rerun-if-changed=build.rs");
}
