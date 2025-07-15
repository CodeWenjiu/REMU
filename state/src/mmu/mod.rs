use core::fmt;

remu_macro::mod_flat!(memory, device, mmu);

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Mask{
    None,
    Byte = 1,
    Half = 2,
    Word = 4,
}

use bitflags::bitflags;
bitflags! {
    #[derive(Clone, Debug)]
    pub struct MemoryFlags: u8 {
        const Read      = 1 << 0;
        const Write     = 1 << 1;
        const Execute   = 1 << 2;
    }
}

impl fmt::Display for MemoryFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

#[derive(Debug)]
pub enum MMTarget {
    Memory(Memory),
    Device(Device),
}

impl MMTarget {
    pub fn read(&mut self, addr: u32, mask: Mask) -> u32 {
        match self {
            MMTarget::Memory(memory) => memory.read(addr, mask),
            MMTarget::Device(device) => device.read(addr, mask),
        }
    }
    pub fn write(&mut self, addr: u32, data: u32, mask: Mask) {
        match self {
            MMTarget::Memory(memory) => memory.write(addr, data, mask),
            MMTarget::Device(device) => device.write(addr, data, mask),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MMTargetType {
    Memory,
    Device,
}

impl fmt::Display for MMTarget {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MMTarget::Memory(_) => write!(f, "Memory"),
            MMTarget::Device(_) => write!(f, "Device"),
        }
    }
}
