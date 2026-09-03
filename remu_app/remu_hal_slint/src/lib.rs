//! Slint UI abstraction for remu apps — works on both RISC-V and host.
//!
//! Wraps the `slint` crate and installs a custom [`Platform`] that drives
//! Slint's software renderer into the remu display framebuffer, feeding
//! keyboard / mouse events from the remu input devices.
//!
//! Because remu_hal exposes the same framebuffer + input API on both host and
//! embedded, this crate is *not* platform-differentiated: the same software
//! renderer path is used everywhere. On host the window is provided by
//! remu_hal's existing winit/softbuffer backend; on embedded the remu display
//! device drives it.
//!
//! Apps keep their `.slint` files and compile them via `slint-build` in their
//! own `build.rs`, and use `slint::include_modules!()` in their crate. This
//! crate re-exports `slint` and pins the shared (workspace) feature set, so
//! apps only add a thin `slint.workspace = true` dependency for
//! `include_modules!` to resolve.

#![cfg_attr(any(target_arch = "riscv32", target_arch = "riscv64"), no_std)]

extern crate alloc;

// Re-export the slint crate so apps can use `slint::include_modules!()`.
pub use slint;

remu_macro::mod_pub!(platform);

pub use platform::SlintApp;
