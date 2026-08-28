use remu_isa::isa::RvIsa;

use crate::bus::{Bus, BusError, BusObserver};

impl<I: RvIsa, O: BusObserver> Bus<I, O> {
    /// Fast path: only the RAM dcache hit. Returns `Some(v)` on hit, `None` on
    /// miss with **no refill side-effect** (see `Memory::read_8_hit`). Inlined
    /// into the hot path so a hit costs no function call and no error
    /// construction. On `None`, callers must fall back to `read_8_slow_err`,
    /// which does the full (single) refill and error path.
    #[inline(always)]
    pub fn read_8_fast(&mut self, addr: usize) -> Option<u8> {
        self.memory.read_8_hit(addr)
    }

    /// Slow path: full lookup (memory miss -> device -> unmapped) that constructs
    /// a detailed `BusError` (incl. backtrace). Out-of-line so the large error
    /// path does not bloat / de-inline the hot path.
    #[inline(never)]
    pub fn read_8_slow_err(&mut self, addr: usize) -> Result<u8, BusError> {
        self.read_8_impl::<true>(addr)
    }

    #[inline(always)]
    pub(crate) fn read_8_impl<const NOTIFY_OBSERVER: bool>(
        &mut self,
        addr: usize,
    ) -> Result<u8, BusError> {
        if let Some(v) = self.memory.read_8(addr) {
            return Ok(v);
        }

        if let Some(d) = self.find_device_mut(addr..addr + 1) {
            let val = d.1.read_8(addr - d.0)?;
            if O::ENABLED && NOTIFY_OBSERVER {
                self.observer.on_mmio_read_8(addr, val);
            }
            return Ok(val);
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn read_8(&mut self, addr: usize) -> Result<u8, BusError> {
        self.read_8_impl::<true>(addr)
    }

    /// Fast path: only the RAM dcache hit. See `read_8_fast`.
    #[inline(always)]
    pub fn read_16_fast(&mut self, addr: usize) -> Option<u16> {
        self.memory.read_16_hit(addr)
    }

    /// Slow path: full lookup with detailed error. See `read_8_slow_err`.
    #[inline(never)]
    pub fn read_16_slow_err(&mut self, addr: usize) -> Result<u16, BusError> {
        self.read_16_impl::<true>(addr)
    }

    #[inline(always)]
    pub(crate) fn read_16_impl<const NOTIFY_OBSERVER: bool>(
        &mut self,
        addr: usize,
    ) -> Result<u16, BusError> {
        if let Some(v) = self.memory.read_16(addr) {
            return Ok(v);
        }

        if let Some(d) = self.find_device_mut(addr..addr + 2) {
            let val = d.1.read_16(addr - d.0)?;
            if O::ENABLED && NOTIFY_OBSERVER {
                self.observer.on_mmio_read_16(addr, val);
            }
            return Ok(val);
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn read_16(&mut self, addr: usize) -> Result<u16, BusError> {
        self.read_16_impl::<true>(addr)
    }

    /// Fast path: only the RAM dcache hit. Returns `Some(v)` on hit, `None` on
    /// miss with **no refill side-effect**. Inlined into the hot path so a hit
    /// costs no function call and no error construction. On `None`, callers must
    /// fall back to `read_32_slow_err` for the full (single refill) error path.
    #[inline(always)]
    pub fn read_32_fast(&mut self, addr: usize) -> Option<u32> {
        self.memory.read_32_hit(addr)
    }

    /// Slow path: full lookup (memory miss -> device -> unmapped) that constructs
    /// a detailed `BusError` (incl. backtrace). Out-of-line so the large error
    /// path does not bloat / de-inline the hot path.
    #[inline(never)]
    pub fn read_32_slow_err(&mut self, addr: usize) -> Result<u32, BusError> {
        self.read_32_impl::<true>(addr)
    }

    #[inline(always)]
    pub(crate) fn read_32_impl<const NOTIFY_OBSERVER: bool>(
        &mut self,
        addr: usize,
    ) -> Result<u32, BusError> {
        if let Some(v) = self.memory.read_32(addr) {
            return Ok(v);
        }

        if let Some(d) = self.find_device_mut(addr..addr + 4) {
            let val = d.1.read_32(addr - d.0)?;
            if O::ENABLED && NOTIFY_OBSERVER {
                self.observer.on_mmio_read_32(addr, val);
            }
            return Ok(val);
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn read_32(&mut self, addr: usize) -> Result<u32, BusError> {
        self.read_32_impl::<true>(addr)
    }

    /// Fast path: only the RAM dcache hit. See `read_32_fast`.
    #[inline(always)]
    pub fn read_64_fast(&mut self, addr: usize) -> Option<u64> {
        self.memory.read_64_hit(addr)
    }

    /// Slow path: full lookup with detailed error. See `read_32_slow_err`.
    #[inline(never)]
    pub fn read_64_slow_err(&mut self, addr: usize) -> Result<u64, BusError> {
        self.read_64_impl::<true>(addr)
    }

    #[inline(always)]
    pub(crate) fn read_64_impl<const NOTIFY_OBSERVER: bool>(
        &mut self,
        addr: usize,
    ) -> Result<u64, BusError> {
        if let Some(v) = self.memory.read_64(addr) {
            return Ok(v);
        }

        if let Some(d) = self.find_device_mut(addr..addr + 8) {
            let val = d.1.read_64(addr - d.0)?;
            if O::ENABLED && NOTIFY_OBSERVER {
                self.observer.on_mmio_read_64(addr, val);
            }
            return Ok(val);
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn read_64(&mut self, addr: usize) -> Result<u64, BusError> {
        self.read_64_impl::<true>(addr)
    }

    /// Fast path: only the RAM dcache hit. See `read_32_fast`.
    #[inline(always)]
    pub fn read_128_fast(&mut self, addr: usize) -> Option<u128> {
        self.memory.read_128_hit(addr)
    }

    /// Slow path: full lookup with detailed error. See `read_32_slow_err`.
    #[inline(never)]
    pub fn read_128_slow_err(&mut self, addr: usize) -> Result<u128, BusError> {
        self.read_128_impl::<true>(addr)
    }

    #[inline(always)]
    pub(crate) fn read_128_impl<const NOTIFY_OBSERVER: bool>(
        &mut self,
        addr: usize,
    ) -> Result<u128, BusError> {
        if let Some(v) = self.memory.read_128(addr) {
            return Ok(v);
        }

        if let Some(d) = self.find_device_mut(addr..addr + 16) {
            let val = d.1.read_128(addr - d.0)?;
            if O::ENABLED && NOTIFY_OBSERVER {
                self.observer.on_mmio_read_128(addr, val);
            }
            return Ok(val);
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn read_128(&mut self, addr: usize) -> Result<u128, BusError> {
        self.read_128_impl::<true>(addr)
    }

    #[inline(always)]
    pub fn read_bytes(&mut self, addr: usize, buf: &mut [u8]) -> Result<(), BusError> {
        if self.memory.read_bytes(addr, buf).is_some() {
            return Ok(());
        }

        Err(BusError::unmapped(addr))
    }

    /// Fast path: write to RAM dcache hit directly. On hit returns `Some(())`;
    /// `None` means miss (MMIO / unmapped) with **no refill side-effect** (see
    /// `Memory::write_8_hit`). Memory-write observer notification is kept (it is
    /// compile-time eliminated for observers with `ENABLED == false`).
    #[inline(always)]
    pub fn write_8_fast(&mut self, addr: usize, value: u8) -> Option<()> {
        let hit = self.memory.write_8_hit(addr, value);
        if hit.is_some() && O::ENABLED {
            self.observer.on_mem_write_8(addr, value);
        }
        hit
    }

    /// Slow path: full lookup with detailed error. See `read_32_slow_err`.
    #[inline(never)]
    pub fn write_8_slow_err(&mut self, addr: usize, value: u8) -> Result<(), BusError> {
        self.write_8_impl::<true>(addr, value)
    }

    #[inline(always)]
    pub(crate) fn write_8_impl<const NOTIFY_OBSERVER: bool>(
        &mut self,
        addr: usize,
        value: u8,
    ) -> Result<(), BusError> {
        if self.memory.write_8(addr, value).is_some() {
            if O::ENABLED && NOTIFY_OBSERVER {
                self.observer.on_mem_write_8(addr, value);
            }

            return Ok(());
        }

        if let Some(d) = self.find_device_mut(addr..addr + 1) {
            d.1.write_8(addr - d.0, value)?;

            if O::ENABLED && NOTIFY_OBSERVER {
                self.observer.on_mmio_write_8(addr, value);
            }

            return Ok(());
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn write_8(&mut self, addr: usize, value: u8) -> Result<(), BusError> {
        self.write_8_impl::<true>(addr, value)
    }

    /// Fast path: write to RAM (dcache hit). See `write_8_fast`.
    #[inline(always)]
    pub fn write_16_fast(&mut self, addr: usize, value: u16) -> Option<()> {
        let hit = self.memory.write_16_hit(addr, value);
        if hit.is_some() && O::ENABLED {
            self.observer.on_mem_write_16(addr, value);
        }
        hit
    }

    /// Slow path: full lookup with detailed error.
    #[inline(never)]
    pub fn write_16_slow_err(&mut self, addr: usize, value: u16) -> Result<(), BusError> {
        self.write_16_impl::<true>(addr, value)
    }

    #[inline(always)]
    pub(crate) fn write_16_impl<const NOTIFY_OBSERVER: bool>(
        &mut self,
        addr: usize,
        value: u16,
    ) -> Result<(), BusError> {
        if self.memory.write_16(addr, value).is_some() {
            if O::ENABLED && NOTIFY_OBSERVER {
                self.observer.on_mem_write_16(addr, value);
            }

            return Ok(());
        }

        if let Some(d) = self.find_device_mut(addr..addr + 2) {
            d.1.write_16(addr - d.0, value)?;

            if O::ENABLED && NOTIFY_OBSERVER {
                self.observer.on_mmio_write_16(addr, value);
            }

            return Ok(());
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn write_16(&mut self, addr: usize, value: u16) -> Result<(), BusError> {
        self.write_16_impl::<true>(addr, value)
    }

    /// Fast path: write to RAM (dcache hit). See `write_8_fast`.
    #[inline(always)]
    pub fn write_32_fast(&mut self, addr: usize, value: u32) -> Option<()> {
        let hit = self.memory.write_32_hit(addr, value);
        if hit.is_some() && O::ENABLED {
            self.observer.on_mem_write_32(addr, value);
        }
        hit
    }

    /// Slow path: full lookup with detailed error.
    #[inline(never)]
    pub fn write_32_slow_err(&mut self, addr: usize, value: u32) -> Result<(), BusError> {
        self.write_32_impl::<true>(addr, value)
    }

    #[inline(always)]
    pub(crate) fn write_32_impl<const NOTIFY_OBSERVER: bool>(
        &mut self,
        addr: usize,
        value: u32,
    ) -> Result<(), BusError> {
        if self.memory.write_32(addr, value).is_some() {
            if O::ENABLED && NOTIFY_OBSERVER {
                self.observer.on_mem_write_32(addr, value);
            }

            return Ok(());
        }

        if let Some(d) = self.find_device_mut(addr..addr + 4) {
            d.1.write_32(addr - d.0, value)?;

            if O::ENABLED && NOTIFY_OBSERVER {
                self.observer.on_mmio_write_32(addr, value);
            }

            return Ok(());
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn write_32(&mut self, addr: usize, value: u32) -> Result<(), BusError> {
        self.write_32_impl::<true>(addr, value)
    }

    /// Write 32-bit to memory/MMIO without notifying the observer (e.g. for breakpoint patch).
    #[inline(always)]
    pub fn write_32_no_observer(&mut self, addr: usize, value: u32) -> Result<(), BusError> {
        self.write_32_impl::<false>(addr, value)
    }

    /// Write 32-bit with byte mask. Reads current value (no observer), merges in masked bytes, writes once.
    #[inline(always)]
    pub fn write_32_masked(&mut self, addr: usize, data: u32, wstrb: u32) -> Result<(), BusError> {
        let old = self.read_32_impl::<false>(addr).unwrap_or(0);
        let mut merged = old;
        for i in 0..4 {
            if (wstrb & (1 << i)) != 0 {
                let mask = 0xff << (i * 8);
                merged = (merged & !mask) | (data & mask);
            }
        }
        self.write_32(addr, merged)
    }

    /// Fast path: write to RAM (dcache hit). See `write_8_fast`.
    #[inline(always)]
    pub fn write_64_fast(&mut self, addr: usize, value: u64) -> Option<()> {
        let hit = self.memory.write_64_hit(addr, value);
        if hit.is_some() && O::ENABLED {
            self.observer.on_mem_write_64(addr, value);
        }
        hit
    }

    /// Slow path: full lookup with detailed error.
    #[inline(never)]
    pub fn write_64_slow_err(&mut self, addr: usize, value: u64) -> Result<(), BusError> {
        self.write_64_impl::<true>(addr, value)
    }

    #[inline(always)]
    pub(crate) fn write_64_impl<const NOTIFY_OBSERVER: bool>(
        &mut self,
        addr: usize,
        value: u64,
    ) -> Result<(), BusError> {
        if self.memory.write_64(addr, value).is_some() {
            if O::ENABLED && NOTIFY_OBSERVER {
                self.observer.on_mem_write_64(addr, value);
            }

            return Ok(());
        }

        if let Some(d) = self.find_device_mut(addr..addr + 8) {
            d.1.write_64(addr - d.0, value)?;

            if O::ENABLED && NOTIFY_OBSERVER {
                self.observer.on_mmio_write_64(addr, value);
            }

            return Ok(());
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn write_64(&mut self, addr: usize, value: u64) -> Result<(), BusError> {
        self.write_64_impl::<true>(addr, value)
    }

    /// Fast path: write to RAM (dcache hit). See `write_8_fast`.
    #[inline(always)]
    pub fn write_128_fast(&mut self, addr: usize, value: u128) -> Option<()> {
        let hit = self.memory.write_128_hit(addr, value);
        if hit.is_some() && O::ENABLED {
            self.observer.on_mem_write_128(addr, value);
        }
        hit
    }

    /// Slow path: full lookup with detailed error.
    #[inline(never)]
    pub fn write_128_slow_err(&mut self, addr: usize, value: u128) -> Result<(), BusError> {
        self.write_128_impl::<true>(addr, value)
    }

    #[inline(always)]
    pub(crate) fn write_128_impl<const NOTIFY_OBSERVER: bool>(
        &mut self,
        addr: usize,
        value: u128,
    ) -> Result<(), BusError> {
        if self.memory.write_128(addr, value).is_some() {
            if O::ENABLED && NOTIFY_OBSERVER {
                self.observer.on_mem_write_128(addr, value);
            }

            return Ok(());
        }

        if let Some(d) = self.find_device_mut(addr..addr + 16) {
            d.1.write_128(addr - d.0, value)?;

            if O::ENABLED && NOTIFY_OBSERVER {
                self.observer.on_mmio_write_128(addr, value);
            }

            return Ok(());
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn write_128(&mut self, addr: usize, value: u128) -> Result<(), BusError> {
        self.write_128_impl::<true>(addr, value)
    }

    #[inline(always)]
    pub fn write_bytes(&mut self, addr: usize, buf: &[u8]) -> Result<(), BusError> {
        if self.memory.write_bytes(addr, buf).is_some() {
            return Ok(());
        }

        Err(BusError::unmapped(addr))
    }
}
