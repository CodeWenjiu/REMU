//! Hardware abstraction for remu simulator devices.
//!
//! Provides embedded-hal / embedded-io trait implementations for devices
//! exposed by remu (UART 16550, etc.) and helpers for program exit.
//! Re-exports riscv-rt entry, panic-halt, and traits so app crates need minimal deps.
//!
//! For logging from `no_std` code, use `write_fmt`, or import `remu_hal::println` / `remu_hal::print`
//! and use `println!(...)` / `print!(...)`; output goes to the default UART.
//!
//! # Time
//!
//! [`read_mtime`] reads CLINT `mtime` at `0x0200_0000 + 0xBFF8` (10 MHz in remu); see module `time`.
//!
//! # Trap handling
//!
//! This crate defines riscv-rt global symbols `ExceptionHandler` and `DefaultHandler`: on an
//! unhandled machine trap they print `mcause`, `mepc`, and `mtval` to the default UART, then panic.

#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;
extern crate panic_halt;

remu_macro::mod_flat!(addresses, print, time, trap, cpu, heap, exit, uart);

pub use alloc::{boxed::Box, string::String, vec::Vec};
pub use core::fmt::Write as FmtWrite;
pub use embedded_io::Write;
pub use riscv_rt::entry;
