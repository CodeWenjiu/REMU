//! `print!` / `println!` macros: the one output channel every platform gets.

use core::fmt;

/// Write formatted output to the default output. Flushes on host.
#[inline]
pub fn write_fmt(args: fmt::Arguments<'_>) {
    // Device-arm platforms (bare-metal MMIO UART, rcore syscall fd 1): the
    // writer's internals differ, but both are exposed as the same stateless
    // backend `Uart16550` object, so the code is shared.
    #[cfg(any(target_os = "none", target_os = "rcore"))]
    {
        use core::fmt::Write;
        #[cfg(target_os = "none")]
        use remu_hal_embedded::Uart16550;
        #[cfg(target_os = "rcore")]
        use remu_hal_rcore::Uart16550;
        let mut uart = Uart16550::default_base();
        let _ = uart.write_fmt(args);
    }
    #[cfg(any(unix, windows))]
    {
        use std::io::Write;
        let mut out = std::io::stdout();
        let _ = out.write_fmt(args);
        let _ = out.flush();
    }
}

/// Print with no trailing newline.
#[macro_export]
macro_rules! print {
    ($($t:tt)*) => {{
        $crate::write_fmt(format_args!($($t)*));
    }};
}

/// Print with trailing newline.
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
