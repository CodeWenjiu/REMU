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

remu_macro::mod_prv!(addresses, app_args, cpu, heap, time, trap, uart, exit);

pub use alloc::{boxed::Box, string::String, vec::Vec};
pub use app_args::app_args;
pub use core::fmt::Write as FmtWrite;
pub use embedded_io::Write;
pub use exit::{exit_failure, exit_success};
pub use heap::init;
pub use riscv_rt::entry;
pub use time::{MTIME_TICK_HZ, read_mtime};
pub use uart::Uart16550;
