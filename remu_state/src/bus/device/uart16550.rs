//! UART 16550 compatible device (8 registers, MMIO byte/word access).
//!
//! Register layout (DLAB in LCR bit 7):
//! - 0: RBR(r)/THR(w) or DLL when DLAB=1
//! - 1: IER(r/w) or DLM when DLAB=1
//! - 2: IIR(r)/FCR(w)
//! - 3: LCR(r/w)
//! - 4: MCR(r/w)
//! - 5: LSR(r) — bit5=THRE, bit6=TEMT
//! - 6: MSR(r)
//! - 7: scratch(r/w)

use std::io::{self, Write};

use crate::bus::{device::DeviceAccess, BusError};

/// LSR bit: Transmitter Holding Register Empty (always ready in emulation).
const LSR_THRE: u8 = 1 << 5;
/// LSR bit: Transmitter Empty.
const LSR_TEMT: u8 = 1 << 6;
/// LCR bit: Divisor Latch Access.
const LCR_DLAB: u8 = 1 << 7;

pub struct Uart16550 {
    lcr: u8,
    ier: u8,
    mcr: u8,
}

impl Uart16550 {
    pub fn new() -> Self {
        Self {
            lcr: 0,
            ier: 0,
            mcr: 0,
        }
    }

    #[inline(always)]
    fn dlab(&self) -> bool {
        self.lcr & LCR_DLAB != 0
    }
}

impl Default for Uart16550 {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceAccess for Uart16550 {
    fn name(&self) -> &str {
        "uart16550"
    }

    fn size(&self) -> usize {
        8
    }

    fn read_8(&mut self, offset: usize) -> Result<u8, BusError> {
        Ok(match offset {
            0 => {
                if self.dlab() {
                    0 // DLL (divisor latch low), no baud emulation
                } else {
                    0 // RBR: no input
                }
            }
            1 => {
                if self.dlab() {
                    0 // DLM
                } else {
                    self.ier
                }
            }
            2 => 0x01, // IIR: no interrupt pending
            3 => self.lcr,
            4 => self.mcr,
            5 => LSR_THRE | LSR_TEMT, // always ready to send
            6 => 0,                   // MSR
            7 => 0,                   // scratch
            _ => 0,
        })
    }

    fn write_8(&mut self, offset: usize, value: u8) -> Result<(), BusError> {
        match offset {
            0 => {
                if !self.dlab() {
                    let stdout = io::stdout();
                    let mut handle = stdout.lock();
                    let bytes = if value == b'\n' { b"\r\n" } else { std::slice::from_ref(&value) };
                    handle
                        .write_all(bytes)
                        .map_err(|_| BusError::IoError(std::backtrace::Backtrace::capture()))?;
                    handle
                        .flush()
                        .map_err(|_| BusError::IoError(std::backtrace::Backtrace::capture()))?;
                }
                // THR; when DLAB=1 this is DLL, ignore
            }
            1 => {
                if !self.dlab() {
                    self.ier = value;
                }
                // else DLM, ignore
            }
            2 => {} // FCR, ignore
            3 => self.lcr = value,
            4 => self.mcr = value,
            5 => {} // LSR read-only
            6 => {} // MSR read-only
            7 => {} // scratch, optional store
            _ => {}
        }
        Ok(())
    }

    fn read_32(&mut self, offset: usize) -> Result<u32, BusError> {
        if offset > 4 {
            return Err(BusError::UnsupportedAccessWidth(
                32,
                std::backtrace::Backtrace::capture(),
            ));
        }
        let b0 = self.read_8(offset)? as u32;
        let b1 = self.read_8(offset + 1)? as u32;
        let b2 = self.read_8(offset + 2)? as u32;
        let b3 = self.read_8(offset + 3)? as u32;
        Ok(b0 | (b1 << 8) | (b2 << 16) | (b3 << 24))
    }

    fn write_32(&mut self, offset: usize, value: u32) -> Result<(), BusError> {
        if offset > 4 {
            return Err(BusError::UnsupportedAccessWidth(
                32,
                std::backtrace::Backtrace::capture(),
            ));
        }
        self.write_8(offset, value as u8)?;
        self.write_8(offset + 1, (value >> 8) as u8)?;
        self.write_8(offset + 2, (value >> 16) as u8)?;
        self.write_8(offset + 3, (value >> 24) as u8)?;
        Ok(())
    }
}
