use remu_types::isa::RvIsa;

use crate::bus::{Bus, BusFault, BusObserver};

impl<I: RvIsa> Bus<I> {
    #[inline(always)]
    pub fn read_8(&mut self, addr: usize) -> Result<u8, BusFault> {
        if let Some(m) = self.find_memory_mut(addr..addr + 1) {
            return Ok(m.read_8(addr));
        }

        if let Some(d) = self.find_device_mut(addr..addr + 1) {
            return Ok(unsafe { *d.1.read(1, addr - d.0)?.get_unchecked(0) });
        }

        Err(BusFault::Unmapped { addr })
    }

    #[inline(always)]
    pub fn read_16(&mut self, addr: usize) -> Result<u16, BusFault> {
        if let Some(m) = self.find_memory_mut(addr..addr + 2) {
            return Ok(m.read_16(addr));
        }

        Err(BusFault::Unmapped { addr })
    }

    #[inline(always)]
    pub fn read_32(&mut self, addr: usize) -> Result<u32, BusFault> {
        if let Some(m) = self.find_memory_mut(addr..addr + 4) {
            return Ok(m.read_32(addr));
        }

        Err(BusFault::Unmapped { addr })
    }

    #[inline(always)]
    pub fn read_64(&mut self, addr: usize) -> Result<u64, BusFault> {
        if let Some(m) = self.find_memory_mut(addr..addr + 8) {
            return Ok(m.read_64(addr));
        }

        Err(BusFault::Unmapped { addr })
    }

    #[inline(always)]
    pub fn read_128(&mut self, addr: usize) -> Result<u128, BusFault> {
        if let Some(m) = self.find_memory_mut(addr..addr + 16) {
            return Ok(m.read_128(addr));
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
            if O::ENABLED {
                obs.on_mem_write(addr, 1, value.into());
            }

            return Ok(m.write_8(addr, value));
        }

        if let Some(d) = self.find_device_mut(addr..addr + 1) {
            d.1.write(1, addr - d.0, &value.to_le_bytes())?;
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
            if O::ENABLED {
                obs.on_mem_write(addr, 2, value.into());
            }

            return Ok(m.write_16(addr, value));
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
            if O::ENABLED {
                obs.on_mem_write(addr, 4, value.into());
            }

            return Ok(m.write_32(addr, value));
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
            if O::ENABLED {
                obs.on_mem_write(addr, 8, value.into());
            }

            return Ok(m.write_64(addr, value));
        }

        Err(BusFault::Unmapped { addr })
    }

    #[inline(always)]
    pub fn write_128(&mut self, addr: usize, value: u128) -> Result<(), BusFault> {
        if let Some(m) = self.find_memory_mut(addr..addr + 16) {
            return Ok(m.write_128(addr, value));
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
