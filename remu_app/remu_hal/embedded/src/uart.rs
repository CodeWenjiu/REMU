//! UART 16550 driver implementing embedded_io::Write.

use core::convert::Infallible;
use embedded_io::ErrorType;

use crate::addresses::UART16550_BASE;

/// UART 16550 device for remu.
#[derive(Debug, Clone, Copy)]
pub struct Uart16550 {
    base: usize,
}

impl Uart16550 {
    #[inline]
    pub const fn new(base: usize) -> Self {
        Self { base }
    }

    #[inline]
    pub const fn default_base() -> Self {
        Self::new(UART16550_BASE)
    }
}

impl ErrorType for Uart16550 {
    type Error = Infallible;
}

impl embedded_io::Write for Uart16550 {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let thr = self.base;
        for &b in buf {
            unsafe { core::ptr::write_volatile(thr as *mut u8, b) };
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl core::fmt::Write for Uart16550 {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        use embedded_io::Write;
        self.write(s.as_bytes()).map_err(|_| core::fmt::Error)?;
        Ok(())
    }
}
