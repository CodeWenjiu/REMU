use std::path::Path;
use std::str::FromStr;

use remu_types::isa::IsaSpec;

use crate::isa_shorthand::{self, NamedExtension, ParsedAppShorthand};

pub const ZVE32_SHORT: &str = "riscv32im_zve32x_zvl128b";
pub const ZVE32_REMU_ISA: &str = "riscv32im_zve32x_zvl128b";
pub const ZVE32_TARGET_RUSTFLAGS: &str = "-C target-feature=+zve32x,+zvl128b";
pub const ZVE32_CARGO_TRIPLE: &str = "riscv32im-unknown-none-elf";
pub const CARGO_TARGET_RUSTFLAGS_RV32I_ENV: &str =
    "CARGO_TARGET_RISCV32I_UNKNOWN_NONE_ELF_RUSTFLAGS";
pub const CARGO_TARGET_RUSTFLAGS_RV32IM_ENV: &str =
    "CARGO_TARGET_RISCV32IM_UNKNOWN_NONE_ELF_RUSTFLAGS";
pub const CARGO_TARGET_DIR_SUBDIR_APP: &str = "app";
pub const CARGO_TARGET_DIR_SUBDIR_ZVE: &str = "app_zve32x";
pub const REMU_ISA_ENV: &str = "REMU_ISA";
/// When set (value ignored), `print run-remu` appends `WJ_CUS0_ISA_SUFFIX` (`_wjCus0`) to `--isa`.
pub const EXISA0_ENV: &str = "EXISA0";
pub const WJ_CUS0_ISA_SUFFIX: &str = "_wjCus0";

#[derive(Debug, Clone)]
pub struct CargoTarget {
    pub triple_or_json: String,
    pub needs_json_target_spec: bool,
    pub zve: bool,
    /// When `zve`, which `CARGO_TARGET_*_RUSTFLAGS` receives [`ZVE32_TARGET_RUSTFLAGS`].
    pub zve_cargo_rustflags_env: Option<&'static str>,
    /// Passed to `remu_cli --isa` via [`REMU_ISA_ENV`] so it matches the built ELF.
    pub remu_isa: Option<String>,
}

impl CargoTarget {
    fn try_from_parsed(parsed: ParsedAppShorthand) -> Result<Self, String> {
        let triple = format!("{}-unknown-none-elf", parsed.base_prefix);
        let (zve, zve_cargo_rustflags_env, remu_isa) = match parsed.extensions.as_slice() {
            [] => (false, None, None),
            [NamedExtension::Zve32xZvl128b] => {
                if parsed.base_prefix == "riscv32imac" {
                    return Err(
                        "xtask: _zve32x_zvl128b is not supported with riscv32imac in app shorthands"
                            .into(),
                    );
                }
                let env = match parsed.base_prefix.as_str() {
                    "riscv32i" => Some(CARGO_TARGET_RUSTFLAGS_RV32I_ENV),
                    "riscv32im" => Some(CARGO_TARGET_RUSTFLAGS_RV32IM_ENV),
                    _ => None,
                };
                let env = env.ok_or_else(|| {
                    format!(
                        "xtask: _zve32x_zvl128b with base {:?} has no CARGO_TARGET_*_RUSTFLAGS mapping",
                        parsed.base_prefix
                    )
                })?;
                let isa = format!("{}_zve32x_zvl128b", parsed.base_prefix);
                (true, Some(env), Some(isa))
            }
            [NamedExtension::WjCus0] => {
                let isa = format!("{}_wjCus0", parsed.base_prefix);
                (false, None, Some(isa))
            }
            _ => {
                return Err(format!(
                    "xtask: app target base {:?} with extensions {:?}: only one named extension is allowed (_zve32x_zvl128b or _wjCus0); combining them is not supported by remu yet",
                    parsed.base_prefix, parsed.extensions
                ));
            }
        };

        if let Some(ref s) = remu_isa {
            IsaSpec::from_str(s).map_err(|e| {
                format!("xtask: invalid ISA string {s:?} derived from app target shorthand: {e}")
            })?;
        }

        Ok(Self {
            triple_or_json: triple,
            needs_json_target_spec: false,
            zve,
            zve_cargo_rustflags_env,
            remu_isa,
        })
    }
}

pub fn merge_cargo_target_rustflags(env_key: &str, fragment: &str) -> String {
    match std::env::var(env_key) {
        Ok(s) if !s.trim().is_empty() => format!("{s} {fragment}"),
        _ => fragment.to_string(),
    }
}

/// Prefer [`merge_cargo_target_rustflags`].
#[inline]
pub fn merge_cargo_target_rv32im_rustflags(fragment: &str) -> String {
    merge_cargo_target_rustflags(CARGO_TARGET_RUSTFLAGS_RV32IM_ENV, fragment)
}

pub fn cargo_target_dir_subdir(zve: bool) -> &'static str {
    if zve {
        CARGO_TARGET_DIR_SUBDIR_ZVE
    } else {
        CARGO_TARGET_DIR_SUBDIR_APP
    }
}

pub fn resolve_for_workspace_root(workspace_root: &Path, key: &str) -> Result<CargoTarget, String> {
    if key.ends_with(".json") {
        let s = resolve_json_path(workspace_root, key);
        return Ok(CargoTarget {
            triple_or_json: s,
            needs_json_target_spec: true,
            zve: false,
            zve_cargo_rustflags_env: None,
            remu_isa: None,
        });
    }
    if key.contains('-') {
        return Ok(CargoTarget {
            triple_or_json: key.to_string(),
            needs_json_target_spec: false,
            zve: false,
            zve_cargo_rustflags_env: None,
            remu_isa: None,
        });
    }
    if let Some(parsed) = isa_shorthand::parse_riscv_app_shorthand(key)? {
        return CargoTarget::try_from_parsed(parsed);
    }
    Ok(CargoTarget {
        triple_or_json: expand_builtin(key),
        needs_json_target_spec: false,
        zve: false,
        zve_cargo_rustflags_env: None,
        remu_isa: None,
    })
}

pub fn resolve_for_hal_dir(key: &str) -> Result<CargoTarget, String> {
    if key.ends_with(".json") {
        return Ok(CargoTarget {
            triple_or_json: key.to_string(),
            needs_json_target_spec: true,
            zve: false,
            zve_cargo_rustflags_env: None,
            remu_isa: None,
        });
    }
    if key.contains('-') {
        return Ok(CargoTarget {
            triple_or_json: key.to_string(),
            needs_json_target_spec: false,
            zve: false,
            zve_cargo_rustflags_env: None,
            remu_isa: None,
        });
    }
    if let Some(parsed) = isa_shorthand::parse_riscv_app_shorthand(key)? {
        return CargoTarget::try_from_parsed(parsed);
    }
    Ok(CargoTarget {
        triple_or_json: expand_builtin(key),
        needs_json_target_spec: false,
        zve: false,
        zve_cargo_rustflags_env: None,
        remu_isa: None,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wj_shorthand_to_cargo_target() {
        let parsed = isa_shorthand::parse_riscv_app_shorthand("riscv32im_wjCus0")
            .unwrap()
            .unwrap();
        let ct = CargoTarget::try_from_parsed(parsed).unwrap();
        assert_eq!(ct.triple_or_json, "riscv32im-unknown-none-elf");
        assert!(!ct.zve);
        assert_eq!(ct.remu_isa.as_deref(), Some("riscv32im_wjCus0"));
    }

    #[test]
    fn zve_wj_combo_rejected() {
        let parsed = isa_shorthand::parse_riscv_app_shorthand("riscv32im_zve32x_zvl128b_wjCus0")
            .unwrap()
            .unwrap();
        assert!(CargoTarget::try_from_parsed(parsed).is_err());
    }
}
