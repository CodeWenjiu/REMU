// Shared crate root for remu_app binaries (referenced from each app's
// `Cargo.toml`, no per-app copy needed):
//
//   [[bin]]
//   name = "remu_app_<name>"
//   path = "../remu_hal/embedded/templates/app_entry.rs"   // (nest deeper if needed)
//
// The embedded targets need `#![no_std]` / `#![no_main]`, which must live in
// the crate root source (Rust cannot inject them from a dependency, a
// Cargo.toml setting, or a build script). Keeping this shell shared means the
// *application* `main.rs` is pure business logic: app authors write a plain
// `fn main` with `#[remu_hal::entry]` (which already adapts itself to the
// target platform) and never touch this file.
//
// Caveat: inner crate attributes (e.g. `#![allow(...)]`) and inner doc
// comments (`//!`) cannot live in `main.rs` (it is `include!`d, so no inner
// attributes or inner docs are permitted there); put crate-wide attributes
// here instead.
#![cfg_attr(any(target_arch = "riscv32", target_arch = "riscv64"), no_std, no_main)]

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs"));
