use super::{BaseApi, MMUApi, Mask, MemoryApi};

pub struct Memory {
    memory: Box<[u8]>,

    length: u32,
}

impl Memory {
    pub fn new(length: u32) -> Self {
        Memory {
            memory: vec![0; length as usize].into_boxed_slice(),
            length,
        }
    }
}

impl BaseApi for Memory {
    fn read(&mut self, addr: u32, mask: Mask) -> u32 {
        let addr = addr as usize;
        match mask {
            Mask::Byte => self.memory[addr] as u32,
            Mask::Half => {
                let mut bytes = [0u8; 2];
                bytes.copy_from_slice(&self.memory[addr..addr+2]);
                u16::from_le_bytes(bytes) as u32
            }
            Mask::Word | Mask::None => {
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&self.memory[addr..addr+4]);
                u32::from_le_bytes(bytes)
            }
        }
    }

    fn write(&mut self, addr: u32, data: u32, mask: Mask) {
        let addr = addr as usize;
        match mask {
            Mask::Byte => self.memory[addr] = data as u8,
            Mask::Half => {
                let bytes = (data as u16).to_le_bytes();
                self.memory[addr..addr+2].copy_from_slice(&bytes);
            }
            Mask::Word | Mask::None => {
                let bytes = data.to_le_bytes();
                self.memory[addr..addr+4].copy_from_slice(&bytes);
            }
        }
    }
}

impl MemoryApi for Memory {
    fn load(&mut self, addr: u32, data: &[u8]) {
        let addr = addr as usize;
        self.memory[addr..addr+data.len()].copy_from_slice(data);
    }

    fn get_length(&self) -> u32 {
        self.length
    }
}

impl MMUApi for Memory {}
