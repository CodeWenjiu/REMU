use std::path::Path;

pub const ZVE32_SHORT: &str = "riscv32im_zve32x_zvl128b";
pub const ZVE32_REMU_ISA: &str = "riscv32im_zve32x_zvl128b";
pub const ZVE32_TARGET_RUSTFLAGS: &str = "-C target-feature=+zve32x,+zvl128b";
pub const ZVE32_CARGO_TRIPLE: &str = "riscv32im-unknown-none-elf";
pub const CARGO_TARGET_RUSTFLAGS_RV32IM_ENV: &str =
    "CARGO_TARGET_RISCV32IM_UNKNOWN_NONE_ELF_RUSTFLAGS";
pub const CARGO_TARGET_DIR_SUBDIR_APP: &str = "app";
pub const CARGO_TARGET_DIR_SUBDIR_ZVE: &str = "app_zve32x";
pub const REMU_ISA_ENV: &str = "REMU_ISA";

#[derive(Debug, Clone)]
pub struct CargoTarget {
    pub triple_or_json: String,
    pub needs_json_target_spec: bool,
    pub zve: bool,
}

pub fn is_zve_short(key: &str) -> bool {
    key == ZVE32_SHORT
}

pub fn merge_cargo_target_rv32im_rustflags(fragment: &str) -> String {
    match std::env::var(CARGO_TARGET_RUSTFLAGS_RV32IM_ENV) {
        Ok(s) if !s.trim().is_empty() => format!("{s} {fragment}"),
        _ => fragment.to_string(),
    }
}

pub fn cargo_target_dir_subdir(zve: bool) -> &'static str {
    if zve {
        CARGO_TARGET_DIR_SUBDIR_ZVE
    } else {
        CARGO_TARGET_DIR_SUBDIR_APP
    }
}

pub fn resolve_for_workspace_root(workspace_root: &Path, key: &str) -> CargoTarget {
    if key.ends_with(".json") {
        let s = resolve_json_path(workspace_root, key);
        return CargoTarget {
            triple_or_json: s,
            needs_json_target_spec: true,
            zve: false,
        };
    }
    if is_zve_short(key) {
        return CargoTarget {
            triple_or_json: ZVE32_CARGO_TRIPLE.into(),
            needs_json_target_spec: false,
            zve: true,
        };
    }
    CargoTarget {
        triple_or_json: expand_builtin(key),
        needs_json_target_spec: false,
        zve: false,
    }
}

pub fn resolve_for_hal_dir(key: &str) -> CargoTarget {
    if key.ends_with(".json") {
        return CargoTarget {
            triple_or_json: key.to_string(),
            needs_json_target_spec: true,
            zve: false,
        };
    }
    if is_zve_short(key) {
        return CargoTarget {
            triple_or_json: ZVE32_CARGO_TRIPLE.into(),
            needs_json_target_spec: false,
            zve: true,
        };
    }
    CargoTarget {
        triple_or_json: expand_builtin(key),
        needs_json_target_spec: false,
        zve: false,
    }
}

fn resolve_json_path(workspace_root: &Path, key: &str) -> String {
    let p = Path::new(key);
    if p.is_absolute() {
        return key.to_string();
    }
    if key.contains('/') {
        return key.to_string();
    }
    workspace_root
        .join("remu_hal")
        .join(key)
        .to_string_lossy()
        .into_owned()
}

pub fn expand_builtin(target: &str) -> String {
    if target.contains('-') {
        target.to_string()
    } else {
        format!("{target}-unknown-none-elf")
    }
}

pub fn artifact_dir_name(cargo_target: &str) -> String {
    Path::new(cargo_target)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(cargo_target)
        .to_string()
}
