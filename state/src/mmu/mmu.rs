use owo_colors::OwoColorize;

use super::{Mask, Memory, MemoryFlags, MmtApi};

pub struct MMU {
    memory_map: Vec<(String, u32, u32, MemoryFlags, Box<dyn MmtApi>)>,
}

#[derive(Debug, snafu::Snafu)]
pub enum MMUError {
    #[snafu(display("memory region conflict: {} [{:#x} : {:#x}] vs {} [{:#x} : {:#x}]", name_first, region_first.0, region_first.1, name_second, region_second.0, region_second.1))]
    MMioRegionConflict {name_first: String, region_first: (u32, u32), name_second: String, region_second: (u32, u32)},
}
pub type MMUResult<T, E = MMUError> = std::result::Result<T, E>;

impl MMU {
    pub fn new() -> Self {
        MMU {
            memory_map: Vec::new(),
        }
    }

    pub fn add_memory(&mut self, base: u32, length: u32, name: &str, flag: MemoryFlags) -> MMUResult<()> {
        for (name_, base_, length_, _, _) in &self.memory_map {
            if  base >= *base_ && 
                base < *base_ + *length_ || 
                base + length > *base_ && 
                base + length <= *base_ + *length_
            {
                return Err(MMUError::MMioRegionConflict { 
                    name_first: name.to_string(), 
                    region_first: (base, base + length), 
                    name_second: name_.to_string(), 
                    region_second: (*base_, *base_ + *length_) 
                });
            }
        }
        
        self.memory_map.push((name.to_string(), base, length, flag, Box::new(Memory::new(length))));

        Ok(())
    }

    pub fn show_memory_map(&self) {
        for (name, base, length, flag, _) in &self.memory_map {
            println!("{}\t [{:#010x} : {:#010x}] [{}]", 
                name.purple(), base.green(), (base + length).red(), format!("{}", flag.blue())
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
