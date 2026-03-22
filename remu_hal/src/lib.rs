//! Hardware abstraction for remu simulator devices.
//!
//! Provides embedded-hal / embedded-io trait implementations for devices
//! exposed by remu (UART 16550, etc.) and helpers for program exit.
//! Re-exports riscv-rt entry, panic-halt, and traits so app crates need minimal deps.
//!
//! For logging from `no_std` code, use `write_fmt`, or import `remu_hal::println` / `remu_hal::print`
//! and use `println!(...)` / `print!(...)`; output goes to the default UART.

#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;
extern crate panic_halt;

mod addresses;
mod print;
remu_macro::mod_pub!(cpu, heap, exit, uart);

pub use print::write_fmt;
pub use cpu::pre_main_init;
pub use heap::init;
pub use exit::{exit_failure, exit_success};
pub use uart::Uart16550;

pub use alloc::{boxed::Box, string::String, vec::Vec};
pub use core::fmt::Write as FmtWrite;
pub use embedded_io::Write;
pub use riscv_rt::entry;
