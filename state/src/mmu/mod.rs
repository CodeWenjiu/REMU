use core::fmt;

remu_macro::mod_flat!(memory, mmu);

#[derive(Debug, PartialEq, Clone)]
pub enum Mask{
    None,
    Byte = 1,
    Half = 2,
    Word = 4,
}

use bitflags::bitflags;
use logger::Logger;
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

pub trait BaseApi {
    fn read(&mut self, _addr: u32, _mask: Mask) -> u32 {
        Logger::todo();
        0
    }

    fn write(&mut self, _addr: u32, _data: u32, _mask: Mask) {
        Logger::todo();
    }
}

pub trait MemoryApi {
    fn load(&mut self, _addr: u32, _data: &[u8]) {
        Logger::todo();
    }
    fn get_length(&self) -> u32 {
        Logger::todo();
        0
    }
}

pub trait MMUApi : BaseApi + MemoryApi{}
