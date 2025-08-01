use std::{cell::{RefCell, RefMut}, rc::Rc};

use owo_colors::OwoColorize;
use remu_macro::log_error;
use remu_utils::{ProcessError, ProcessResult};

use super::{MMTarget, MMTargetType, Mask, Memory, MemoryFlags, Device};

#[derive(Clone)]
pub struct MMU {
    memory_map: Vec<(String, u32, u32, MemoryFlags, Rc<RefCell<MMTarget>>)>,
}

#[derive(Debug, snafu::Snafu)]
pub enum MMUError {
    #[snafu(display("memory region conflict: {} [{:#x} : {:#x}] vs {} [{:#x} : {:#x}]", name_first, region_first.0, region_first.1, name_second, region_second.0, region_second.1))]
    MMioRegionConflict {name_first: String, region_first: (u32, u32), name_second: String, region_second: (u32, u32)},

    #[snafu(display("memory unmapped: {:#010x}", addr))]
    MemoryUnmapped {addr: u32},

    #[snafu(display("memory unknowned: {}", name))]
    MemoryUnkowned {name: String},

    #[snafu(display("load out of range: {:#010x} : {:#010x}", addr, addr + len))]
    LoadOutOfRange {addr: u32, len: u32},

    #[snafu(display("memory unreadable: {:#010x}", addr))]
    MemoryUnreadable {addr: u32},

    #[snafu(display("memory unwritable: {:#010x}", addr))]
    MemoryUnwritable {addr: u32},

    #[snafu(display("memory unexecutable: {:#010x}", addr))]
    MemoryUnexecutable {addr: u32},
}
pub type MMUResult<T, E = MMUError> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct RegionConfiguration {
    pub name: String,
    pub base: u32,
    pub size: u32,
    pub flag: MemoryFlags,
    pub mmtype: MMTargetType,
}

impl MMU {
    pub fn new() -> Self {
        MMU {
            memory_map: Vec::new(),
        }
    }

    pub fn add_region(&mut self, region: &RegionConfiguration) -> MMUResult<()> {
        // Check for conflicts with existing memory regions
        let (base, length, name, flag, r#type) = (region.base, region.size, &region.name, region.flag.clone(), region.mmtype);
        for (name_, base_, length_, _, _, ) in &self.memory_map {
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
        let new_region = 
            (name.to_string(), base, length, flag, 
            Rc::new(RefCell::new(match r#type {
                MMTargetType::Memory => MMTarget::Memory(Memory::new(length)),
                MMTargetType::Device => MMTarget::Device(Device::new(name)),
            })));
        
        // Find the correct position to insert based on base address
        let position = self.memory_map.iter()
            .position(|(_, b, _, _, _, )| *b > base)
            .unwrap_or(self.memory_map.len());
        
        // Insert at the correct position to maintain sorted order
        self.memory_map.insert(position, new_region);

        Ok(())
    }

    pub fn show_memory_map(&self) {
        for (name, base, length, flag, target) in &self.memory_map {
            println!("{}\t [{:#010x} : {:#010x}] [{}] [{}]", 
                name.purple(), base.green(), (base + length).red(), format!("{}", target.borrow().blue()), format!("{}", flag.blue())
            );
        }
    }

    fn find_memory_region(&self, addr: u32) 
        -> MMUResult<(RefMut<'_, Memory>, u32, &MemoryFlags)> {
        for (_, base, length, flag, memory) in &self.memory_map {
            if addr >= *base && addr < *base + *length {
                // Check type first to avoid borrowing issues
                let is_memory = matches!(&*memory.borrow(), MMTarget::Memory(_));
                
                if is_memory {
                    // Map the RefMut<MMTargetType> to RefMut<Memory>
                    let mem_ref = RefMut::map(memory.borrow_mut(), |m| {
                        match m {
                            MMTarget::Memory(inner) => inner,
                            _ => unreachable!(),
                        }
                    });
                    return Ok((mem_ref, addr - *base, flag));
                } else {
                    return Err(MMUError::MemoryUnmapped { addr });
                }
            }
        }
        Err(MMUError::MemoryUnmapped { addr })
    }

    fn find_region_byname(&self, name: &str) 
        -> MMUResult<(RefMut<'_, MMTarget>, &MemoryFlags)> {
        for (name_, _, _, flag, memory) in &self.memory_map {
            if name == name_ {
                return Ok((memory.borrow_mut(), flag));
            }
        }
        Err(MMUError::MemoryUnkowned { name: name.to_string() })
    }

    fn find_region(&self, addr: u32) -> MMUResult<(RefMut<'_, MMTarget>, u32, &MemoryFlags)> {
        for (_, base, length, flag, memory) in &self.memory_map {
            if addr >= *base && addr < *base + *length {
                return Ok((memory.borrow_mut(), addr - *base, flag));
            }
        }
        Err(MMUError::MemoryUnmapped { addr })
    }

    pub fn is_dev(&self, addr: u32) -> MMUResult<bool> {
        for (_, base, length, _, memory) in &self.memory_map {
            if addr >= *base && addr < *base + *length {
                return Ok(matches!(&*memory.borrow(), MMTarget::Device(_)));
            }
        }
        Err(MMUError::MemoryUnmapped { addr })
    }

    pub fn read(&mut self, addr: u32, mask: Mask) -> MMUResult<u32> {
        let (mut memory, offset, flags) = self.find_region(addr)?;
        
        if !flags.contains(MemoryFlags::Read) {
            return Err(MMUError::MemoryUnreadable { addr });
        }
        
        Ok(memory.read(offset, mask))
    }

    pub fn read_memory(&mut self, addr: u32, mask: Mask) -> MMUResult<u32> {
        let (mut memory, offset, flags) = self.find_memory_region(addr)?;
        
        if !flags.contains(MemoryFlags::Read) {
            return Err(MMUError::MemoryUnreadable { addr });
        }
        
        Ok(memory.read(offset, mask))
    }

    pub fn read_by_name(&mut self, name: &str, offset: u32, mask: Mask) -> MMUResult<u32> {
        let (mut memory, flags) = self.find_region_byname(name)?;
        
        if !flags.contains(MemoryFlags::Read) {
            return Err(MMUError::MemoryUnreadable { addr: offset });
        }
        
        Ok(memory.read(offset, mask))
    }

    pub fn write(&mut self, addr: u32, data: u32, mask: Mask) -> MMUResult<()> {
        let (mut memory, offset, flags) = self.find_region(addr)?;
        
        if !flags.contains(MemoryFlags::Write) {
            return Err(MMUError::MemoryUnwritable { addr });
        }
        
        memory.write(offset, data, mask);
        Ok(())
    }

    pub fn write_by_name(&mut self, name: &str, offset: u32, data: u32, mask: Mask) -> MMUResult<()> {
        let (mut memory, flags) = self.find_region_byname(name)?;
        
        if !flags.contains(MemoryFlags::Write) {
            return Err(MMUError::MemoryUnwritable { addr: offset });
        }
        
        memory.write(offset, data, mask);
        Ok(())
    }

    pub fn inst_fetch(&mut self, addr: u32) -> MMUResult<u32> {
        let (mut memory, offset, flags) = self.find_region(addr)?;
        
        if !flags.contains(MemoryFlags::Execute) {
            return Err(MMUError::MemoryUnexecutable { addr });
        }
        
        Ok(memory.read(offset, Mask::Word))
    }

    pub fn load(&mut self, addr: u32, data: &[u8]) -> MMUResult<()> {
        let (mut memory, offset, _) = self.find_memory_region(addr)?;
        
        if (offset + data.len() as u32) > memory.get_length() {
            return Err(MMUError::LoadOutOfRange { addr, len: data.len() as u32 });
        }
            
        memory.load(offset, data);
        Ok(())
    }

    pub fn check(&mut self, mem_diff_msg: Vec<(u32, u32)>) -> ProcessResult<()> {
        for (addr, data) in mem_diff_msg {
            let (mut memory, offset, flags) = self.find_memory_region(addr).unwrap();

            if !flags.contains(MemoryFlags::Read) {
                println!("Memory unreadable at {:#010x}", addr);
                return Err(ProcessError::Recoverable);
            }
            
            let read_data = memory.read(offset, Mask::Word);
            if read_data != data {
                log_error!(format!("Memory mismatch at {:#010x}: expected {:#010x}, got {:#010x}", addr, read_data, data));
                return Err(ProcessError::Recoverable);
            }
        }
        Ok(())
    }
}
