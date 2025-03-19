use owo_colors::OwoColorize;

use super::{Mask, Memory, MemoryFlags, MmtApi};

pub struct MMU {
    memory_map: Vec<(String, u32, u32, MemoryFlags, Box<dyn MmtApi>)>,
}

impl MMU {
    pub fn new() -> Self {
        MMU {
            memory_map: Vec::new(),
        }
    }

    pub fn add_memory(&mut self, base: u32, length: u32, name: &str, flag: MemoryFlags) {
        self.memory_map.push((name.to_string(), base, length, flag, Box::new(Memory::new(length))));
    }

    pub fn show_memory_map(&self) {
        for (name, base, length, flag, _) in &self.memory_map {
            println!("{}: range: [{:#x} : {:#x}] [{}]", 
                name.purple(), base.green(), length.red(), format!("{}", flag.blue())
            );
        }
    }

    pub fn read(&mut self, addr: u32, mask: Mask) -> u32 {
        for (_, base, length, flag, memory) in &mut self.memory_map {
            if  addr >= *base && 
                addr < *base + *length && 
                flag.contains(MemoryFlags::Read) 
            {
                return memory.read(addr - *base, mask);
            }
        }
        0
    }
}
