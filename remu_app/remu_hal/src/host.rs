//! Host (x86_64) equivalents of embedded HAL items.

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

/// No-op heap init on host (global allocator already set up by std).
#[inline]
pub fn init() {}
