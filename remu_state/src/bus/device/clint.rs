//! CLINT (Core Local Interruptor) device — standard RISC-V layout, timing only.
//!
//! No interrupt delivery; registers are read/write as per spec for layout compatibility.
//! mtime is derived from host time at 10 MHz.
//!
//! Layout (single hart):
//! - 0x0000: msip (4 bytes)
//! - 0x4000: mtimecmp (8 bytes)
//! - 0xBFF8: mtime (8 bytes, read-only; value = elapsed host time at 10 MHz)

use std::time::Instant;

use crate::bus::{device::DeviceAccess, BusError};

/// CLINT size per RISC-V platform spec (e.g. SiFive).
const CLINT_SIZE: usize = 0xC000;

/// mtime register offset (64-bit); high 32 bits at +4.
const MTIME_OFF: usize = 0xBFF8;
const MTIME_HIGH_OFF: usize = MTIME_OFF + 4;
/// mtimecmp register offset for hart 0 (64-bit).
const MTIMECMP_OFF: usize = 0x4000;
/// msip register offset for hart 0 (32-bit).
const MSIP_OFF: usize = 0x0000;

fn mtime_ticks_from_elapsed_nanos(nanos: u128) -> u64 {
    // 10 MHz => 10^7 ticks per second; 1 ns => 10^7/10^9 = 1/100 tick => ticks = nanos / 100
    (nanos / 100) as u64
}

pub struct Clint {
    base_instant: Instant,
    msip: u32,
    mtimecmp: u64,
}

impl Clint {
    pub fn new() -> Self {
        Self {
            base_instant: Instant::now(),
            msip: 0,
            mtimecmp: 0,
        }
    }

    fn mtime_now(&self) -> u64 {
        let elapsed = self.base_instant.elapsed();
        let nanos = elapsed.as_nanos();
        mtime_ticks_from_elapsed_nanos(nanos)
    }
}

impl Default for Clint {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceAccess for Clint {
    fn name(&self) -> &str {
        "clint"
    }

    fn size(&self) -> usize {
        CLINT_SIZE
    }

    fn read_32(&mut self, offset: usize) -> Result<u32, BusError> {
        Ok(match offset {
            MSIP_OFF => self.msip,
            MTIME_OFF => self.mtime_now() as u32,
            MTIME_HIGH_OFF => (self.mtime_now() >> 32) as u32,
            _ => 0,
        })
    }

    fn write_32(&mut self, offset: usize, value: u32) -> Result<(), BusError> {
        match offset {
            MSIP_OFF => self.msip = value,
            _ => {}
        }
        Ok(())
    }

    fn read_64(&mut self, offset: usize) -> Result<u64, BusError> {
        Ok(match offset {
            MTIMECMP_OFF => self.mtimecmp,
            MTIME_OFF => self.mtime_now(),
            _ => 0,
        })
    }

    fn write_64(&mut self, offset: usize, value: u64) -> Result<(), BusError> {
        match offset {
            MTIMECMP_OFF => self.mtimecmp = value,
            MTIME_OFF => {} // read-only in spec; ignore
            _ => {}
        }
        Ok(())
    }
}
