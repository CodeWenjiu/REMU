//! Hardware abstraction for remu apps — works on both RISC-V and host.
//!
//! On embedded RISC-V, delegates to [`remu_hal_embedded`]. On host, provides
//! stdout-based equivalents with the same API.
//!
//! # Portable app pattern
//!
//! ```ignore
//! #![cfg_attr(target_arch = "riscv32", no_std, no_main)]
//! extern crate alloc;
//!
//! #[cfg_attr(target_arch = "riscv32", remu_hal::entry)]
//! fn main() -> ! {
//!     remu_hal::init();
//!     let mut uart = remu_hal::Uart16550::default_base();
//!     let _ = writeln!(uart, "hello");
//!     let _ = remu_hal::Write::flush(&mut uart);
//!     remu_hal::exit_success()
//! }
//! ```

#![cfg_attr(any(target_arch = "riscv32", target_arch = "riscv64"), no_std)]

extern crate alloc;

// ── Platform-specific backend ──
#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
mod host;

// ── Re-exports (all platforms) ──
remu_macro::mod_prv!(print);
pub use alloc::{boxed::Box, string::String, vec::Vec};
pub use core::fmt::Write as FmtWrite;
pub use print::write_fmt;

#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
pub use embedded_io::Write;

#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
pub use remu_hal_embedded::{
    MTIME_TICK_HZ, Uart16550, app_args, entry, exit_failure, exit_success, read_mtime,
};

#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
pub use host::{MTIME_TICK_HZ, Stdout as Uart16550, read_mtime};

// ── Display device (both platforms; host returns no-op stubs) ──
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
pub use remu_hal_embedded::{
    DisplaySize, FB_BASE, FB_HEIGHT, FB_WIDTH, KeyState, MouseState, display_alive, fb_base,
    frame_done, put_pixel, read_disp_h, read_disp_size, read_disp_w, read_key, read_key_buttons,
    read_key_code, read_key_down, read_key_text, read_key_valid, read_mouse, read_mouse_buttons,
    read_mouse_x, read_mouse_y,
};

#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
pub use host::{
    DisplaySize, FB_HEIGHT, FB_WIDTH, KeyState, MouseState, display_alive, fb_base, frame_done,
    put_pixel, read_disp_h, read_disp_size, read_disp_w, read_key, read_key_buttons, read_key_code,
    read_key_down, read_key_text, read_key_valid, read_mouse, read_mouse_buttons, read_mouse_x,
    read_mouse_y,
};

// ── Safe init (wraps unsafe embedded init) ──
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
pub fn init() {
    unsafe { remu_hal_embedded::init() };
}
#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
pub use host::init;

// ── Exit (both platforms) ──
#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
pub fn exit_success() -> ! {
    std::process::exit(0);
}
#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
pub fn exit_failure() -> ! {
    std::process::exit(1);
}
