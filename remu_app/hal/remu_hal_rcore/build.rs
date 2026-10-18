//! User program build script: publish the user linker script as `link.x`.
//!
//! Same mechanism as `rcore_kernel` / `remu_firmware`: the workspace
//! `.cargo/config.toml` passes `-Tlink.x` to every riscv build (both
//! `target_os = "none"` and the custom rcore target), so this crate writes
//! its own script into OUT_DIR to provide that name. Because this crate is a
//! target-specific dependency of `remu_hal` (`target_os = "rcore"` only), it
//! never coexists with riscv-rt's `link.x` in the same build.

use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let script = include_str!("linker.ld");
    std::fs::write(out.join("link.x"), script).expect("write link.x to OUT_DIR");
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=linker.ld");
}
