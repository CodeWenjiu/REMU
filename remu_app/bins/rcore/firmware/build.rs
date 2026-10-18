//! Firmware build script: publish the firmware linker script as `link.x`.
//!
//! Same mechanism as `rcore_kernel`: the workspace `.cargo/config.toml`
//! passes `-Tlink.x` to every riscv-none build, so this crate writes its own
//! script into OUT_DIR to provide that name (the firmware does not use
//! riscv-rt). See `kernel/build.rs` for details.

use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let script = include_str!("linker.ld");
    std::fs::write(out.join("link.x"), script).expect("write link.x to OUT_DIR");
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=linker.ld");
}
