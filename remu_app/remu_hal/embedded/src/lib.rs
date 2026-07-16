//! Pure embedded HAL for RISC-V targets running on remu.
//!
//! Provides UART 16550, SiFive test finisher exit, CLINT timer,
//! heap init via embedded-alloc, and M-mode trap handlers.
//!
//! Only compiles for `riscv32*` / `riscv64*` targets; for host support
//! use the parent [`remu_hal`] crate which wraps this one.

#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;
extern crate panic_halt;

remu_macro::mod_flat!(addresses, app_args, cpu, heap, time, trap, uart, exit);

pub use alloc::{boxed::Box, string::String, vec::Vec};
pub use core::fmt::Write as FmtWrite;
pub use embedded_io::Write;
pub use heap::init;
pub use riscv_rt::entry;
pub use uart::Uart16550;
