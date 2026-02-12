use remu_types::isa::RvIsa;

use crate::bus::{Bus, BusError, BusObserver};

impl<I: RvIsa, O: BusObserver> Bus<I, O> {
    #[inline(always)]
    pub fn read_8(&mut self, addr: usize) -> Result<u8, BusError> {
        if let Some(v) = self.memory.read_8(addr) {
            return Ok(v);
        }

        if let Some(d) = self.find_device_mut(addr..addr + 1) {
            let val = d.1.read_8(addr - d.0)?;
            if O::ENABLED {
                self.observer.on_mmio_read_8(addr, val);
            }
            return Ok(val);
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn read_16(&mut self, addr: usize) -> Result<u16, BusError> {
        if let Some(v) = self.memory.read_16(addr) {
            return Ok(v);
        }

        if let Some(d) = self.find_device_mut(addr..addr + 2) {
            let val = d.1.read_16(addr - d.0)?;
            if O::ENABLED {
                self.observer.on_mmio_read_16(addr, val);
            }
            return Ok(val);
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn read_32(&mut self, addr: usize) -> Result<u32, BusError> {
        if let Some(v) = self.memory.read_32(addr) {
            return Ok(v);
        }

        if let Some(d) = self.find_device_mut(addr..addr + 4) {
            let val = d.1.read_32(addr - d.0)?;
            if O::ENABLED {
                self.observer.on_mmio_read_32(addr, val);
            }
            return Ok(val);
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn read_64(&mut self, addr: usize) -> Result<u64, BusError> {
        if let Some(v) = self.memory.read_64(addr) {
            return Ok(v);
        }

        if let Some(d) = self.find_device_mut(addr..addr + 8) {
            let val = d.1.read_64(addr - d.0)?;
            if O::ENABLED {
                self.observer.on_mmio_read_64(addr, val);
            }
            return Ok(val);
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn read_128(&mut self, addr: usize) -> Result<u128, BusError> {
        if let Some(v) = self.memory.read_128(addr) {
            return Ok(v);
        }

        if let Some(d) = self.find_device_mut(addr..addr + 16) {
            let val = d.1.read_128(addr - d.0)?;
            if O::ENABLED {
                self.observer.on_mmio_read_128(addr, val);
            }
            return Ok(val);
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn read_bytes(&mut self, addr: usize, buf: &mut [u8]) -> Result<(), BusError> {
        if self.memory.read_bytes(addr, buf).is_some() {
            return Ok(());
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn write_8(&mut self, addr: usize, value: u8) -> Result<(), BusError> {
        if self.memory.write_8(addr, value).is_some() {
            if O::ENABLED {
                self.observer.on_mem_write_8(addr, value);
            }

            return Ok(());
        }

        if let Some(d) = self.find_device_mut(addr..addr + 1) {
            d.1.write_8(addr - d.0, value)?;

            if O::ENABLED {
                self.observer.on_mmio_write_8(addr, value);
            }

            return Ok(());
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn write_16(&mut self, addr: usize, value: u16) -> Result<(), BusError> {
        if self.memory.write_16(addr, value).is_some() {
            if O::ENABLED {
                self.observer.on_mem_write_16(addr, value);
            }

            return Ok(());
        }

        if let Some(d) = self.find_device_mut(addr..addr + 2) {
            d.1.write_16(addr - d.0, value)?;

            if O::ENABLED {
                self.observer.on_mmio_write_16(addr, value);
            }

            return Ok(());
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn write_32(&mut self, addr: usize, value: u32) -> Result<(), BusError> {
        if self.memory.write_32(addr, value).is_some() {
            if O::ENABLED {
                self.observer.on_mem_write_32(addr, value);
            }

            return Ok(());
        }

        if let Some(d) = self.find_device_mut(addr..addr + 4) {
            d.1.write_32(addr - d.0, value)?;

            if O::ENABLED {
                self.observer.on_mmio_write_32(addr, value);
            }

            return Ok(());
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn write_64(&mut self, addr: usize, value: u64) -> Result<(), BusError> {
        if self.memory.write_64(addr, value).is_some() {
            if O::ENABLED {
                self.observer.on_mem_write_64(addr, value);
            }

            return Ok(());
        }

        if let Some(d) = self.find_device_mut(addr..addr + 8) {
            d.1.write_64(addr - d.0, value)?;

            if O::ENABLED {
                self.observer.on_mmio_write_64(addr, value);
            }

            return Ok(());
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn write_128(&mut self, addr: usize, value: u128) -> Result<(), BusError> {
        if self.memory.write_128(addr, value).is_some() {
            if O::ENABLED {
                self.observer.on_mem_write_128(addr, value);
            }

            return Ok(());
        }

        if let Some(d) = self.find_device_mut(addr..addr + 16) {
            d.1.write_128(addr - d.0, value)?;

            if O::ENABLED {
                self.observer.on_mmio_write_128(addr, value);
            }

            return Ok(());
        }

        Err(BusError::unmapped(addr))
    }

    #[inline(always)]
    pub fn write_bytes(&mut self, addr: usize, buf: &[u8]) -> Result<(), BusError> {
        if self.memory.write_bytes(addr, buf).is_some() {
            return Ok(());
        }

        Err(BusError::unmapped(addr))
    }
}
