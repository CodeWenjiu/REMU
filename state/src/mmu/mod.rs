use core::fmt;

use enum_dispatch::enum_dispatch;

remu_macro::mod_flat!(memory, mmu);

#[derive(Debug, PartialEq, Clone)]
pub enum Mask{
    None,
    Byte = 1,
    Half = 2,
    Word = 4,
}

use bitflags::bitflags;
bitflags! {
    #[derive(Clone)]
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

#[enum_dispatch]
pub trait MmtApi {
    fn read(&mut self, addr: u32, mask: Mask) -> u32; // read device will change state
    fn write(&mut self, addr: u32, data: u32, mask: Mask);
}

#[enum_dispatch(MmtApi)]
enum Mmts {
    Memory,
}
