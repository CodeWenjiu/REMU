use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Paths {
    pub hal_dir: PathBuf,
    pub workspace_root: PathBuf,
}

impl Paths {
    pub fn from_env() -> Self {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR must be set (invoke via cargo)");
        Self::from_manifest(&manifest_dir)
    }

    pub fn from_manifest(manifest_dir: impl AsRef<Path>) -> Self {
        let manifest_dir = manifest_dir.as_ref();
        let hal_dir = manifest_dir
            .parent()
            .expect("xtask lives in remu_hal/xtask")
            .to_path_buf();
        let workspace_root = hal_dir
            .parent()
            .expect("remu_hal lives under workspace root")
            .to_path_buf();
        Self {
            hal_dir,
            workspace_root,
        }
    }

    pub fn hal_canonical(&self) -> PathBuf {
        self.hal_dir.canonicalize().expect("remu_hal path")
    }

    pub fn workspace_canonical(&self) -> PathBuf {
        self.workspace_root.canonicalize().expect("workspace root")
    }
}
