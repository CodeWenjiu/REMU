//! Locate, load, and cache `libmujoco.so` via dlopen2 on first use.
//!
//! No build step and no link dependency: the engine is dlopen'd at runtime.
//! Candidate order: `$MUJOCO_DIR/lib/libmujoco.so`,
//! `$MUJOCO_DYNAMIC_LINK_DIR/libmujoco.so`, then the loader's own search
//! (bare `libmujoco.so`, e.g. via `LD_LIBRARY_PATH`).

use std::env;
use std::path::PathBuf;
use std::sync::OnceLock;

use dlopen2::wrapper::Container;

use crate::MujocoError;
use super::ffi::MujocoApi;

/// Minimum accepted engine version (`mj_version()` encoding, e.g. 3014000 for
/// 3.14.0). Anything below 3.x predates the state-signature API used here.
const MIN_VERSION: i32 = 3_000_000;

static MUJOCO: OnceLock<Result<Container<MujocoApi>, String>> = OnceLock::new();

/// The loaded engine handle; loads on first call and caches the result.
pub(crate) fn api() -> Result<&'static Container<MujocoApi>, MujocoError> {
    MUJOCO
        .get_or_init(load)
        .as_ref()
        .map_err(|e| MujocoError::Engine(e.clone()))
}

/// Engine version (`mj_version()`, e.g. 3014000 for 3.14.0).
pub(crate) fn engine_version() -> Result<i32, MujocoError> {
    Ok(unsafe { api()?.version() })
}

pub(crate) fn candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(dir) = env::var_os("MUJOCO_DIR") {
        paths.push(PathBuf::from(dir).join("lib").join("libmujoco.so"));
    }
    if let Some(dir) = env::var_os("MUJOCO_DYNAMIC_LINK_DIR") {
        paths.push(PathBuf::from(dir).join("libmujoco.so"));
    }
    paths.push(PathBuf::from("libmujoco.so"));
    paths
}

fn load() -> Result<Container<MujocoApi>, String> {
    let mut last_err = String::from("no candidates tried");
    for path in candidate_paths() {
        // Safety: no callbacks or constructors of ours run inside the library;
        // only ELF initialization happens at load time.
        match unsafe { Container::<MujocoApi>::load(&path) } {
            Ok(cont) => {
                let version = unsafe { cont.version() };
                if version >= MIN_VERSION {
                    return Ok(cont);
                }
                last_err = format!(
                    "{}: mujoco {version} is too old (need >= 3.x)",
                    path.display()
                );
            }
            Err(e) => last_err = format!("{}: {e}", path.display()),
        }
    }
    Err(format!(
        "cannot load libmujoco.so ({last_err}); run inside the Nix dev shell \
         (MUJOCO_DIR is exported there) or point MUJOCO_DIR at a MuJoCo install"
    ))
}
