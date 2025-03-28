use core::fmt;

remu_macro::mod_flat!(memory, device, mmu);

#[derive(Debug, PartialEq, Clone)]
pub enum Mask{
    None,
    Byte = 1,
    Half = 2,
    Word = 4,
}

use bitflags::bitflags;
use logger::Logger;
use remu_macro::log_todo;
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

#[derive(Clone, Debug)]
pub enum MMTargetType {
    Memory,
    Device,
}

impl fmt::Display for MMTargetType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MMTargetType::Memory => write!(f, "Memory"),
            MMTargetType::Device => write!(f, "Device"),
        }
    }
}

pub trait BaseApi {
    fn read(&mut self, _addr: u32, _mask: Mask) -> u32 {
        log_todo!();
        0
    }

    fn write(&mut self, _addr: u32, _data: u32, _mask: Mask) {
        log_todo!();
    }
}

pub trait MemoryApi {
    fn load(&mut self, _addr: u32, _data: &[u8]) {
        log_todo!();
    }
    fn get_length(&self) -> u32 {
        log_todo!();
        0
    }
}

pub trait MMUApi : BaseApi + MemoryApi{}
