//! Hardware abstraction for remu simulator devices.
//!
//! Provides embedded-hal / embedded-io trait implementations for devices
//! exposed by remu (UART 16550, etc.) and helpers for program exit.
//! Re-exports riscv-rt entry, panic-halt, and traits so app crates need minimal deps.

#![no_std]

extern crate panic_halt;

mod addresses;
mod exit;
mod uart;

pub use embedded_io::Write;
pub use exit::{exit_failure, exit_success};
pub use riscv_rt::entry;
pub use uart::Uart16550;
