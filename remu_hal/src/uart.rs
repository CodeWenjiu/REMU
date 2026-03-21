//! UART 16550 driver implementing embedded_io::Write.
//!
//! Register layout (offset from base):
//! - 0: THR(w) — Transmit Holding Register
//! - 5: LSR(r) — Line Status Register, bit5=THRE (always ready in remu)

use core::convert::Infallible;

use embedded_io::ErrorType;

use crate::addresses::UART16550_BASE;

/// UART 16550 device for remu.
///
/// Implements [embedded_io::Write] for byte output. Uses MMIO to the THR
/// at base+0; remu's emulation has LSR always ready, so no polling needed.
#[derive(Debug, Clone, Copy)]
pub struct Uart16550 {
    base: usize,
}

impl Uart16550 {
    /// Create a UART at the given base address.
    #[inline]
    pub const fn new(base: usize) -> Self {
        Self { base }
    }

    /// Create a UART at remu's default base (0x1000_0000).
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
