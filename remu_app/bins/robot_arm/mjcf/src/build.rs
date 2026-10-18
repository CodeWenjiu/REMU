//! `build.rs` entry point: resolve the model declared by the app's
//! `[package.metadata.remu] arm-model`, then generate `$OUT_DIR/kinematics.rs`.
//!
//! On failure the generated file carries a `compile_error!` so the reason
//! surfaces at the app's `include!` site instead of a bare build-script panic.

use std::fs;
use std::path::{Path, PathBuf};

pub fn run() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");

    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-env-changed=MUJOCO_MENAGERIE_DIR");

    let content = match generate(Path::new(&manifest_dir)) {
        Ok((content, files)) => {
            for file in files {
                println!("cargo:rerun-if-changed={}", file.display());
            }
            content
        }
        Err(e) => {
            println!("cargo:warning=mjcf: {e}");
            format!("compile_error!({:?});\n", format!("mjcf: {e}"))
        }
    };
    let out = Path::new(&out_dir).join("kinematics.rs");
    fs::write(&out, content).unwrap_or_else(|e| panic!("write {}: {e}", out.display()));
}

fn generate(manifest_dir: &Path) -> Result<(String, Vec<PathBuf>), String> {
    let arm_model = arm_model_of(manifest_dir)?;
    let menagerie = std::env::var("MUJOCO_MENAGERIE_DIR").map_err(|_| {
        format!("MUJOCO_MENAGERIE_DIR is not set (run inside the dev shell); needed to resolve `{arm_model}`")
    })?;
    let path = Path::new(&menagerie).join(&arm_model);
    println!("cargo:rerun-if-changed={}", path.display());

    let loaded = crate::parse::load_file(&path)?;
    let model = crate::extract::extract(&loaded.root)?;
    Ok((crate::emit::emit(&model, &arm_model), loaded.files))
}

fn arm_model_of(manifest_dir: &Path) -> Result<String, String> {
    let path = manifest_dir.join("Cargo.toml");
    let text =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let doc: toml::Table = text
        .parse()
        .map_err(|e| format!("cannot parse {}: {e}", path.display()))?;
    doc.get("package")
        .and_then(|p| p.get("metadata"))
        .and_then(|m| m.get("remu"))
        .and_then(|r| r.get("arm-model"))
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .ok_or_else(|| "no `[package.metadata.remu] arm-model` declaration".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_arm_model_from_the_manifest() {
        let dir = std::env::temp_dir().join(format!("mjcf_manifest_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"x\"\n[package.metadata.remu]\narm-model = \"a/b.xml\"\n",
        )
        .unwrap();
        assert_eq!(arm_model_of(&dir).unwrap(), "a/b.xml");
        let _ = fs::remove_dir_all(&dir);
    }
}
