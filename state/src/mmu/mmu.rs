use owo_colors::OwoColorize;

use super::{Mask, Memory, MemoryFlags, MmtApi};

pub struct MMU {
    memory_map: Vec<(String, u32, u32, MemoryFlags, Box<dyn MmtApi>)>,
}

#[derive(Debug, snafu::Snafu)]
pub enum MMUError {
    #[snafu(display("memory region conflict: {} [{:#x} : {:#x}] vs {} [{:#x} : {:#x}]", name_first, region_first.0, region_first.1, name_second, region_second.0, region_second.1))]
    MMioRegionConflict {name_first: String, region_first: (u32, u32), name_second: String, region_second: (u32, u32)},

    #[snafu(display("memory unmapped: {:#010x}", addr))]
    MemoryUnmapped {addr: u32},

    #[snafu(display("memory unreadable: {:#010x}", addr))]
    MemoryUnreadable {addr: u32},

    #[snafu(display("memory unwritable: {:#010x}", addr))]
    MemoryUnwritable {addr: u32},

    #[snafu(display("memory unexecutable: {:#010x}", addr))]
    MemoryUnexecutable {addr: u32},
}
pub type MMUResult<T, E = MMUError> = std::result::Result<T, E>;

impl MMU {
    pub fn new() -> Self {
        MMU {
            memory_map: Vec::new(),
        }
    }

    pub fn add_memory(&mut self, base: u32, length: u32, name: &str, flag: MemoryFlags) -> MMUResult<()> {
        // Check for conflicts with existing memory regions
        for (name_, base_, length_, _, _) in &self.memory_map {
            if !(base + length <= *base_ || base >= *base_ + *length_) {
                return Err(MMUError::MMioRegionConflict { 
                    name_first: name.to_string(), 
                    region_first: (base, base + length), 
                    name_second: name_.to_string(), 
                    region_second: (*base_, *base_ + *length_) 
                });
            }
        }
        
        // Create the new memory region
        let new_region = (name.to_string(), base, length, flag, Box::new(Memory::new(length)) as Box<dyn MmtApi>);
        
        // Find the correct position to insert based on base address
        let position = self.memory_map.iter()
            .position(|(_, b, _, _, _)| *b > base)
            .unwrap_or(self.memory_map.len());
        
        // Insert at the correct position to maintain sorted order
        self.memory_map.insert(position, new_region);

        Ok(())
    }

    pub fn show_memory_map(&self) {
        for (name, base, length, flag, _) in &self.memory_map {
            println!("{}\t [{:#010x} : {:#010x}] [{}]", 
                name.purple(), base.green(), (base + length).red(), format!("{}", flag.blue())
            );
        }
    }

    fn find_memory_region(&mut self, addr: u32) 
        -> MMUResult<(&mut Box<dyn MmtApi>, u32, &MemoryFlags)> {
        for (_, base, length, flag, memory) in &mut self.memory_map {
            if addr >= *base && addr < *base + *length {
                return Ok((memory, addr - *base, flag));
            }
        }
        Err(MMUError::MemoryUnmapped { addr })
    }

    pub fn read(&mut self, addr: u32, mask: Mask) -> MMUResult<u32> {
        let (memory, offset, flags) = self.find_memory_region(addr)?;
        
        if !flags.contains(MemoryFlags::Read) {
            return Err(MMUError::MemoryUnreadable { addr });
        }
        
        Ok(memory.read(offset, mask))
    }

    pub fn write(&mut self, addr: u32, data: u32, mask: Mask) -> MMUResult<()> {
        let (memory, offset, flags) = self.find_memory_region(addr)?;
        
        if !flags.contains(MemoryFlags::Write) {
            return Err(MMUError::MemoryUnwritable { addr });
        }
        
        memory.write(offset, data, mask);
        Ok(())
    }

    pub fn inst_fetch(&mut self, addr: u32) -> MMUResult<u32> {
        let (memory, offset, flags) = self.find_memory_region(addr)?;
        
        if !flags.contains(MemoryFlags::Execute) {
            return Err(MMUError::MemoryUnexecutable { addr });
        }
        
        Ok(memory.read(offset, Mask::Word))
    }
}
