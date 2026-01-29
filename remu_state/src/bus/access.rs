use remu_types::isa::RvIsa;

use crate::bus::{Bus, BusFault, BusObserver};

impl<I: RvIsa> Bus<I> {
    #[inline(always)]
    pub fn read_8(&mut self, addr: usize) -> Result<u8, BusFault> {
        let m = self.find_memory_mut(addr..addr + 1)?;
        Ok(m.read_8(addr))
    }

    #[inline(always)]
    pub fn read_16(&mut self, addr: usize) -> Result<u16, BusFault> {
        let m = self.find_memory_mut(addr..addr + 2)?;
        Ok(m.read_16(addr))
    }

    #[inline(always)]
    pub fn read_32(&mut self, addr: usize) -> Result<u32, BusFault> {
        let m = self.find_memory_mut(addr..addr + 4)?;
        Ok(m.read_32(addr))
    }

    #[inline(always)]
    pub fn read_64(&mut self, addr: usize) -> Result<u64, BusFault> {
        let m = self.find_memory_mut(addr..addr + 8)?;
        Ok(m.read_64(addr))
    }

    #[inline(always)]
    pub fn read_128(&mut self, addr: usize) -> Result<u128, BusFault> {
        let m = self.find_memory_mut(addr..addr + 16)?;
        Ok(m.read_128(addr))
    }

    #[inline(always)]
    pub fn read_bytes(&mut self, addr: usize, buf: &mut [u8]) -> Result<(), BusFault> {
        let m = self.find_memory_mut(addr..addr + buf.len())?;
        Ok(m.read_bytes(addr, buf))
    }

    #[inline(always)]
    pub fn write_8<O: BusObserver>(
        &mut self,
        addr: usize,
        value: u8,
        obs: &mut O,
    ) -> Result<(), BusFault> {
        let m = self.find_memory_mut(addr..addr + 1)?;
        obs.on_mem_write(addr, 1, value.into());

        Ok(m.write_8(addr, value))
    }

    #[inline(always)]
    pub fn write_16<O: BusObserver>(
        &mut self,
        addr: usize,
        value: u16,
        obs: &mut O,
    ) -> Result<(), BusFault> {
        let m = self.find_memory_mut(addr..addr + 2)?;
        obs.on_mem_write(addr, 2, value.into());

        Ok(m.write_16(addr, value))
    }

    #[inline(always)]
    pub fn write_32<O: BusObserver>(
        &mut self,
        addr: usize,
        value: u32,
        obs: &mut O,
    ) -> Result<(), BusFault> {
        let m = self.find_memory_mut(addr..addr + 4)?;
        obs.on_mem_write(addr, 4, value.into());

        Ok(m.write_32(addr, value))
    }

    #[inline(always)]
    pub fn write_64<O: BusObserver>(
        &mut self,
        addr: usize,
        value: u64,
        obs: &mut O,
    ) -> Result<(), BusFault> {
        let m = self.find_memory_mut(addr..addr + 8)?;
        obs.on_mem_write(addr, 8, value.into());

        Ok(m.write_64(addr, value))
    }

    #[inline(always)]
    pub fn write_128(&mut self, addr: usize, value: u128) -> Result<(), BusFault> {
        let m = self.find_memory_mut(addr..addr + 16)?;
        Ok(m.write_128(addr, value))
    }

    #[inline(always)]
    pub fn write_bytes(&mut self, addr: usize, buf: &[u8]) -> Result<(), BusFault> {
        let m = self.find_memory_mut(addr..addr + buf.len())?;
        Ok(m.write_bytes(addr, buf))
    }
}
