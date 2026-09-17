//! Per-app target filtering, declared via `[package.metadata.remu] isas = [...]`
//! in the app's `Cargo.toml`.
//!
//! The feature is **opt-in**: an app without the `isas` entry supports every
//! target (fail-open). An app that declares a list is only built for those
//! targets — `xtask print run-app` / `build-app` refuse to emit a cargo command
//! otherwise, and `print check-app` reports the same verdict for scripts that
//! do not go through xtask (e.g. the `just run-app --platform host` branch).
//!
//! Declared entries use the xtask target vocabulary (`riscv32im`, `riscv32i`,
//! `riscv32imac`, `riscv64*`, with `_zve32x_zvl128b` / `_wjCus0` suffixes when
//! needed) plus the special token `host`.

use std::path::{Path, PathBuf};

use crate::isa_shorthand::{parse_riscv_app_shorthand, ParsedAppShorthand};

/// A target capability, in the same vocabulary as xtask target arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Cap {
    Host,
    Riscv(ParsedAppShorthand),
    /// Not classifiable (`.json` target specs, foreign triples): fail open.
    Other,
}

/// Classify a requested target key. Unrecognized keys become [`Cap::Other`]
/// and are never rejected, preserving pre-feature behavior for power-user
/// targets.
fn requested_cap(key: &str) -> Cap {
    classify(key).unwrap_or(Cap::Other)
}

/// Classify a declared entry from `[package.metadata.remu] isas`. Unlike
/// [`requested_cap`], an unclassifiable entry is an error: declarations must
/// use the xtask target vocabulary.
fn declared_cap(entry: &str) -> Result<Cap, String> {
    classify(entry).ok_or_else(|| {
        format!(
            "invalid `isas` entry {entry:?}: use a target shorthand (riscv32im, riscv64i, riscv32im_zve32x_zvl128b, …) or `host`"
        )
    })
}

fn classify(key: &str) -> Option<Cap> {
    if key == "host" {
        return Some(Cap::Host);
    }
    if let Ok(Some(p)) = parse_riscv_app_shorthand(key) {
        return Some(Cap::Riscv(p));
    }
    // Full triple form: riscv32im-unknown-none-elf (vendor/os/abi ignored).
    let prefix = key.split('-').next().unwrap_or(key);
    if let Ok(Some(p)) = parse_riscv_app_shorthand(prefix) {
        return Some(Cap::Riscv(p));
    }
    None
}

/// Locate the app manifest. App crates live under `remu_app/`, either directly
/// (`remu_app/<name>/`) or nested one level (`remu_app/nes/nes/`). A candidate
/// is kept only if its `package.name` matches `remu_app_<name>`; unknown layouts
/// fall through to `None` (fail-open).
fn app_manifest(workspace_root: &Path, app: &str) -> Option<PathBuf> {
    let base = workspace_root.join("remu_app").join(app);
    let want = format!("remu_app_{app}");
    [base.join("Cargo.toml"), base.join(app).join("Cargo.toml")]
        .into_iter()
        .filter(|c| c.is_file())
        .find(|c| read_package_name(c).as_deref() == Some(want.as_str()))
}

fn read_package_name(manifest: &Path) -> Option<String> {
    let text = std::fs::read_to_string(manifest).ok()?;
    let doc: toml::Table = text.parse().ok()?;
    doc.get("package")?.get("name")?.as_str().map(str::to_owned)
}

/// Parse `[package.metadata.remu] isas = [...]` from the app manifest.
/// `Ok(None)` means no restriction is declared (the app supports every target).
fn declared_caps(manifest: &Path, app: &str) -> Result<Option<Vec<Cap>>, String> {
    let text = std::fs::read_to_string(manifest)
        .map_err(|e| format!("cannot read {}: {e}", manifest.display()))?;
    let doc: toml::Table = text
        .parse()
        .map_err(|e| format!("cannot parse {}: {e}", manifest.display()))?;
    let Some(isas) = doc
        .get("package")
        .and_then(|p| p.get("metadata"))
        .and_then(|m| m.get("remu"))
        .and_then(|r| r.get("isas"))
    else {
        return Ok(None);
    };
    let arr = isas.as_array().ok_or_else(|| {
        format!("app `{app}`: [package.metadata.remu] `isas` must be an array of strings")
    })?;
    let mut caps = Vec::with_capacity(arr.len());
    for item in arr {
        let s = item.as_str().ok_or_else(|| {
            format!("app `{app}`: [package.metadata.remu] `isas` entries must be strings")
        })?;
        caps.push(declared_cap(s).map_err(|e| format!("app `{app}`: {e}"))?);
    }
    Ok(Some(caps))
}

/// Canonical display form of a capability, e.g. `riscv32im_zve32x_zvl128b`.
fn describe(cap: &Cap) -> String {
    match cap {
        Cap::Host => "host".to_string(),
        Cap::Riscv(p) => {
            let mut s = p.base_prefix.clone();
            for ext in &p.extensions {
                s.push('_');
                s.push_str(ext.as_str());
            }
            s
        }
        Cap::Other => "<unknown target>".to_string(),
    }
}

/// Validate that `target_key` is allowed for `app`.
///
/// Fails open (returns `Ok`) when the target is not classifiable (foreign
/// triples, `.json` specs), when the app manifest cannot be located, or when
/// the app declares no restriction.
pub(crate) fn validate_app_target(
    workspace_root: &Path,
    app: &str,
    target_key: &str,
) -> Result<(), String> {
    let target = requested_cap(target_key);
    if target == Cap::Other {
        return Ok(());
    }
    let Some(manifest) = app_manifest(workspace_root, app) else {
        return Ok(());
    };
    let Some(declared) = declared_caps(&manifest, app)? else {
        return Ok(());
    };
    if declared.contains(&target) {
        return Ok(());
    }
    let supported = declared.iter().map(describe).collect::<Vec<_>>().join(", ");
    let rel = manifest.strip_prefix(workspace_root).unwrap_or(&manifest);
    Err(format!(
        "app `{app}` does not support target `{target_key}`\n  supported: {supported}\n  fix: edit [package.metadata.remu] `isas` = [...] in {}",
        rel.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(s: &str) -> ParsedAppShorthand {
        parse_riscv_app_shorthand(s).unwrap().unwrap()
    }

    fn tempdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "remu_xtask_{tag}_{}_{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: PathBuf) {
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn classifies_shorthands_triples_and_host() {
        assert_eq!(requested_cap("host"), Cap::Host);
        assert_eq!(
            requested_cap("riscv32im"),
            requested_cap("riscv32im-unknown-none-elf")
        );
        assert_eq!(
            requested_cap("riscv64imac"),
            requested_cap("riscv64imac-unknown-none-elf")
        );
        assert_eq!(
            requested_cap("riscv32im_zve32x_zvl128b"),
            requested_cap("riscv32im_zve32x_zvl128b-unknown-none-elf")
        );
        assert_eq!(
            requested_cap("riscv64im_wjCus0"),
            requested_cap("riscv64im_WJCUS0")
        );
        assert_eq!(requested_cap("foo.json"), Cap::Other);
        assert_eq!(requested_cap("x86_64-unknown-linux-gnu"), Cap::Other);
    }

    #[test]
    fn declared_caps_parses_metadata() {
        let dir = tempdir("declared_caps");
        let manifest = dir.join("Cargo.toml");
        std::fs::write(
            &manifest,
            "[package]\nname = \"remu_app_x\"\n[package.metadata.remu]\nisas = [\"riscv32im\", \"host\"]\n",
        )
        .unwrap();
        let caps = declared_caps(&manifest, "x").unwrap().unwrap();
        assert_eq!(caps, vec![Cap::Riscv(parsed("riscv32im")), Cap::Host]);
        cleanup(dir);
    }

    #[test]
    fn declared_caps_absent_without_metadata() {
        let dir = tempdir("declared_caps_none");
        let manifest = dir.join("Cargo.toml");
        std::fs::write(&manifest, "[package]\nname = \"remu_app_x\"\n").unwrap();
        assert!(declared_caps(&manifest, "x").unwrap().is_none());
        cleanup(dir);
    }

    #[test]
    fn invalid_declaration_entry_errors() {
        let dir = tempdir("declared_caps_bad");
        let manifest = dir.join("Cargo.toml");
        std::fs::write(
            &manifest,
            "[package]\nname = \"remu_app_x\"\n[package.metadata.remu]\nisas = [\"banana\"]\n",
        )
        .unwrap();
        let err = declared_caps(&manifest, "x").unwrap_err();
        assert!(err.contains("invalid `isas` entry \"banana\""), "{err}");
        cleanup(dir);
    }

    #[test]
    fn validate_allows_without_declaration() {
        let ws = tempdir("no_decl");
        let app_dir = ws.join("remu_app").join("x");
        std::fs::create_dir_all(&app_dir).unwrap();
        std::fs::write(
            app_dir.join("Cargo.toml"),
            "[package]\nname = \"remu_app_x\"\n",
        )
        .unwrap();
        assert!(validate_app_target(&ws, "x", "riscv64im").is_ok());
        assert!(validate_app_target(&ws, "x", "host").is_ok());
        cleanup(ws);
    }

    #[test]
    fn validate_restricts_declared_app() {
        let ws = tempdir("restrict");
        let app_dir = ws.join("remu_app").join("x");
        std::fs::create_dir_all(&app_dir).unwrap();
        std::fs::write(
            app_dir.join("Cargo.toml"),
            "[package]\nname = \"remu_app_x\"\n[package.metadata.remu]\nisas = [\"riscv32im\", \"host\"]\n",
        )
        .unwrap();
        assert!(validate_app_target(&ws, "x", "riscv32im").is_ok());
        assert!(validate_app_target(&ws, "x", "host").is_ok());
        let err = validate_app_target(&ws, "x", "riscv64im").unwrap_err();
        assert!(err.contains("does not support target `riscv64im`"), "{err}");
        assert!(err.contains("supported: riscv32im, host"), "{err}");
        assert!(err.contains("remu_app/x/Cargo.toml"), "{err}");
        // Foreign / undeterminable targets still fail open.
        assert!(validate_app_target(&ws, "x", "foo.json").is_ok());
        cleanup(ws);
    }

    #[test]
    fn validate_finds_nested_app_layout() {
        // remu_app/nes/nes/ pattern.
        let ws = tempdir("nested");
        let app_dir = ws.join("remu_app").join("x").join("x");
        std::fs::create_dir_all(&app_dir).unwrap();
        std::fs::write(
            app_dir.join("Cargo.toml"),
            "[package]\nname = \"remu_app_x\"\n[package.metadata.remu]\nisas = [\"riscv32im\"]\n",
        )
        .unwrap();
        assert!(validate_app_target(&ws, "x", "riscv32im").is_ok());
        let err = validate_app_target(&ws, "x", "riscv64i").unwrap_err();
        assert!(err.contains("remu_app/x/x/Cargo.toml"), "{err}");
        cleanup(ws);
    }
}
