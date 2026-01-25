use remu_types::{RvIsa, Support64, Support128};

use crate::bus::{Bus, BusFault};

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
    pub fn read_bytes(&mut self, addr: usize, buf: &mut [u8]) -> Result<(), BusFault> {
        let m = self.find_memory_mut(addr..addr + buf.len())?;
        Ok(m.read_bytes(addr, buf))
    }

    #[inline(always)]
    pub fn write_8(&mut self, addr: usize, value: u8) -> Result<(), BusFault> {
        let m = self.find_memory_mut(addr..addr + 1)?;
        Ok(m.write_8(addr, value))
    }

    #[inline(always)]
    pub fn write_16(&mut self, addr: usize, value: u16) -> Result<(), BusFault> {
        let m = self.find_memory_mut(addr..addr + 2)?;
        Ok(m.write_16(addr, value))
    }

    #[inline(always)]
    pub fn write_32(&mut self, addr: usize, value: u32) -> Result<(), BusFault> {
        let m = self.find_memory_mut(addr..addr + 4)?;
        Ok(m.write_32(addr, value))
    }

    #[inline(always)]
    pub fn write_bytes(&mut self, addr: usize, buf: &[u8]) -> Result<(), BusFault> {
        let m = self.find_memory_mut(addr..addr + buf.len())?;
        Ok(m.write_bytes(addr, buf))
    }
}

impl<I: RvIsa> Bus<I>
where
    I::XLEN: Support64,
{
    #[inline(always)]
    pub fn read_64(&mut self, addr: usize) -> Result<u64, BusFault> {
        let m = self.find_memory_mut(addr..addr + 8)?;
        Ok(m.read_64(addr))
    }

    #[inline(always)]
    pub fn write_64(&mut self, addr: usize, value: u64) -> Result<(), BusFault> {
        let m = self.find_memory_mut(addr..addr + 8)?;
        Ok(m.write_64(addr, value))
    }
}

impl<I: RvIsa> Bus<I>
where
    I::XLEN: Support128,
{
    #[inline(always)]
    pub fn read_128(&mut self, addr: usize) -> Result<u128, BusFault> {
        let m = self.find_memory_mut(addr..addr + 16)?;
        Ok(m.read_128(addr))
    }

    #[inline(always)]
    pub fn write_128(&mut self, addr: usize, value: u128) -> Result<(), BusFault> {
        let m = self.find_memory_mut(addr..addr + 16)?;
        Ok(m.write_128(addr, value))
    }
}
