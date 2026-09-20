//! Kernel build script: publish the kernel linker script as `link.x`.
//!
//! The workspace `.cargo/config.toml` unconditionally passes `-Tlink.x` to
//! every `riscv*-none-elf` build (a convention inherited from `riscv-rt`,
//! which generates that file). `rcore_kernel` does not use `riscv-rt`, so it
//! would otherwise fail with `cannot find linker script link.x`.
//!
//! Writing our script into `OUT_DIR` as `link.x` and putting `OUT_DIR` on the
//! linker search path makes the name resolve to the *kernel* layout, with no
//! change to shared configuration and no effect on the other crates.

use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let script = include_str!("linker.ld");
    std::fs::write(out.join("link.x"), script).expect("write link.x to OUT_DIR");
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=linker.ld");
}
