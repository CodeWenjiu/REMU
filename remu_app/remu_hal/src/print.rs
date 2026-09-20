//! `print!` / `println!` macros: UART on embedded, stdout on host.

use core::fmt;

/// Write formatted output to the default output. Flushes on host.
#[inline]
pub fn write_fmt(args: fmt::Arguments<'_>) {
    #[cfg(target_os = "none")]
    {
        use crate::Uart16550;
        use core::fmt::Write;
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
