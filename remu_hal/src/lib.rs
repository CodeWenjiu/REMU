//! Hardware abstraction for remu simulator devices.
//!
//! Provides embedded-hal / embedded-io trait implementations for devices
//! exposed by remu (UART 16550, etc.) and helpers for program exit.
//! Re-exports riscv-rt entry, panic-halt, and traits so app crates need minimal deps.

#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;
extern crate panic_halt;

mod addresses;
remu_macro::mod_flat!(cpu, heap, exit, uart);

pub use alloc::{boxed::Box, string::String, vec::Vec};
pub use core::fmt::Write as FmtWrite;
pub use embedded_io::Write;
pub use riscv_rt::entry;
