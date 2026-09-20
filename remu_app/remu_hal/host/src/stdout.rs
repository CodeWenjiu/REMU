//! Stdout writer, API-compatible with the embedded `Uart16550`.

use core::fmt;

/// Stdout writer, API-compatible with `Uart16550`.
pub struct Stdout;

impl Stdout {
    #[inline]
    pub const fn default_base() -> Self {
        Stdout
    }
}

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        use std::io::Write;
        std::io::stdout()
            .write_all(s.as_bytes())
            .map_err(|_| fmt::Error)
    }
}
