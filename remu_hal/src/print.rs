//! `print!` / `println!` macros for `no_std`: format to the default remu UART.
//!
//! Same MMIO device as `Uart16550::default_base()`. Single-hart / no locking;
//! avoid calling from multiple contexts if you add interrupts later.

use core::fmt::{self, Write as _};

use crate::Uart16550;

/// Write formatted output to the default UART (0x1000_0000).
#[inline]
pub fn write_fmt(args: fmt::Arguments<'_>) {
    let mut uart = Uart16550::default_base();
    let _ = uart.write_fmt(args);
}

/// Print to the default UART (no trailing newline).
#[macro_export]
macro_rules! print {
    ($($t:tt)*) => {{
        $crate::write_fmt(format_args!($($t)*));
    }};
}

/// Print to the default UART, with trailing newline.
#[macro_export]
macro_rules! println {
    () => {{
        $crate::write_fmt(format_args!("\n"));
    }};
    ($($t:tt)*) => {{
        $crate::write_fmt(format_args!($($t)*));
        $crate::write_fmt(format_args!("\n"));
    }};
}
