use remu_types::isa::RvIsa;

use crate::bus::{Bus, BusFault, BusObserver};

impl<I: RvIsa> Bus<I> {
    #[inline(always)]
    pub fn read_8(&mut self, addr: usize) -> Result<u8, BusFault> {
        if let Some(m) = self.find_memory_mut(addr..addr + 1) {
            return Ok(m.read_8(addr));
        }

        if let Some(d) = self.find_device_mut(addr..addr + 1) {
            return Ok(d.1.read_8(addr - d.0)?);
        }

        Err(BusFault::Unmapped { addr })
    }

    #[inline(always)]
    pub fn read_16(&mut self, addr: usize) -> Result<u16, BusFault> {
        if let Some(m) = self.find_memory_mut(addr..addr + 2) {
            return Ok(m.read_16(addr));
        }

        if let Some(d) = self.find_device_mut(addr..addr + 1) {
            return Ok(d.1.read_16(addr - d.0)?);
        }

        Err(BusFault::Unmapped { addr })
    }

    #[inline(always)]
    pub fn read_32(&mut self, addr: usize) -> Result<u32, BusFault> {
        if let Some(m) = self.find_memory_mut(addr..addr + 4) {
            return Ok(m.read_32(addr));
        }

        if let Some(d) = self.find_device_mut(addr..addr + 1) {
            return Ok(d.1.read_32(addr - d.0)?);
        }

        Err(BusFault::Unmapped { addr })
    }

    #[inline(always)]
    pub fn read_64(&mut self, addr: usize) -> Result<u64, BusFault> {
        if let Some(m) = self.find_memory_mut(addr..addr + 8) {
            return Ok(m.read_64(addr));
        }

        if let Some(d) = self.find_device_mut(addr..addr + 1) {
            return Ok(d.1.read_64(addr - d.0)?);
        }

        Err(BusFault::Unmapped { addr })
    }

    #[inline(always)]
    pub fn read_128(&mut self, addr: usize) -> Result<u128, BusFault> {
        if let Some(m) = self.find_memory_mut(addr..addr + 16) {
            return Ok(m.read_128(addr));
        }

        if let Some(d) = self.find_device_mut(addr..addr + 1) {
            return Ok(d.1.read_128(addr - d.0)?);
        }

        Err(BusFault::Unmapped { addr })
    }

    #[inline(always)]
    pub fn read_bytes(&mut self, addr: usize, buf: &mut [u8]) -> Result<(), BusFault> {
        if let Some(m) = self.find_memory_mut(addr..addr + buf.len()) {
            return Ok(m.read_bytes(addr, buf));
        }

        Err(BusFault::Unmapped { addr })
    }

    #[inline(always)]
    pub fn write_8<O: BusObserver>(
        &mut self,
        addr: usize,
        value: u8,
        obs: &mut O,
    ) -> Result<(), BusFault> {
        if let Some(m) = self.find_memory_mut(addr..addr + 1) {
            m.write_8(addr, value);

            if O::ENABLED {
                obs.on_mem_write_8(addr, value);
            }

            return Ok(());
        }

        if let Some(d) = self.find_device_mut(addr..addr + 1) {
            d.1.write_8(addr - d.0, value)?;

            if O::ENABLED {
                obs.on_mmio_write_8(addr, value);
            }

            return Ok(());
        }

        Err(BusFault::Unmapped { addr })
    }

    #[inline(always)]
    pub fn write_16<O: BusObserver>(
        &mut self,
        addr: usize,
        value: u16,
        obs: &mut O,
    ) -> Result<(), BusFault> {
        if let Some(m) = self.find_memory_mut(addr..addr + 2) {
            m.write_16(addr, value);

            if O::ENABLED {
                obs.on_mem_write_16(addr, value);
            }

            return Ok(());
        }

        if let Some(d) = self.find_device_mut(addr..addr + 2) {
            d.1.write_16(addr - d.0, value)?;

            if O::ENABLED {
                obs.on_mmio_write_16(addr, value);
            }

            return Ok(());
        }

        Err(BusFault::Unmapped { addr })
    }

    #[inline(always)]
    pub fn write_32<O: BusObserver>(
        &mut self,
        addr: usize,
        value: u32,
        obs: &mut O,
    ) -> Result<(), BusFault> {
        if let Some(m) = self.find_memory_mut(addr..addr + 4) {
            m.write_32(addr, value);

            if O::ENABLED {
                obs.on_mem_write_32(addr, value);
            }

            return Ok(());
        }

        if let Some(d) = self.find_device_mut(addr..addr + 4) {
            d.1.write_32(addr - d.0, value)?;

            if O::ENABLED {
                obs.on_mmio_write_32(addr, value);
            }

            return Ok(());
        }

        Err(BusFault::Unmapped { addr })
    }

    #[inline(always)]
    pub fn write_64<O: BusObserver>(
        &mut self,
        addr: usize,
        value: u64,
        obs: &mut O,
    ) -> Result<(), BusFault> {
        if let Some(m) = self.find_memory_mut(addr..addr + 8) {
            m.write_64(addr, value);

            if O::ENABLED {
                obs.on_mem_write_64(addr, value);
            }

            return Ok(());
        }

        if let Some(d) = self.find_device_mut(addr..addr + 8) {
            d.1.write_64(addr - d.0, value)?;

            if O::ENABLED {
                obs.on_mmio_write_64(addr, value);
            }

            return Ok(());
        }

        Err(BusFault::Unmapped { addr })
    }

    #[inline(always)]
    pub fn write_128<O: BusObserver>(
        &mut self,
        addr: usize,
        value: u128,
        obs: &mut O,
    ) -> Result<(), BusFault> {
        if let Some(m) = self.find_memory_mut(addr..addr + 16) {
            m.write_128(addr, value);

            if O::ENABLED {
                obs.on_mem_write_128(addr, value);
            }

            return Ok(());
        }

        if let Some(d) = self.find_device_mut(addr..addr + 16) {
            d.1.write_128(addr - d.0, value)?;

            if O::ENABLED {
                obs.on_mmio_write_128(addr, value);
            }

            return Ok(());
        }

        Err(BusFault::Unmapped { addr })
    }

    #[inline(always)]
    pub fn write_bytes(&mut self, addr: usize, buf: &[u8]) -> Result<(), BusFault> {
        if let Some(m) = self.find_memory_mut(addr..addr + buf.len()) {
            return Ok(m.write_bytes(addr, buf));
        }

        Err(BusFault::Unmapped { addr })
    }
}
