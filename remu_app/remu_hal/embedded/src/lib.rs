//! Pure embedded HAL for RISC-V targets running on remu.
//!
//! Provides UART 16550, SiFive test finisher exit, CLINT timer,
//! heap init via embedded-alloc, and M-mode trap handlers.
//!
//! Only compiles for `riscv32*` / `riscv64*` targets; for host support
//! use the parent [`remu_hal`] crate which wraps this one.

#![allow(dead_code, unreachable_pub)]
#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;
extern crate panic_halt;

remu_macro::mod_prv!(
    addresses, app_args, cpu, display, heap, time, trap, uart, exit
);

pub use alloc::{boxed::Box, string::String, vec::Vec};
pub use app_args::app_args;
pub use core::fmt::Write as FmtWrite;
pub use display::{
    DisplaySize, FB_BASE, FB_HEIGHT, FB_WIDTH, KeyState, MouseState, display_alive, fb_base,
    frame_done, put_pixel, read_disp_h, read_disp_size, read_disp_w, read_key, read_key_buttons,
    read_key_code, read_key_down, read_key_kind_raw, read_key_seq, read_key_text, read_key_valid,
    read_mouse, read_mouse_buttons, read_mouse_x, read_mouse_y,
};
pub use embedded_io::Write;
pub use exit::{exit_failure, exit_success};
pub use heap::init;
pub use riscv_rt::entry;
pub use time::{MTIME_TICK_HZ, read_mtime};
pub use uart::Uart16550;
