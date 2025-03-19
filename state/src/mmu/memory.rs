use super::{Mask, MmtApi};

pub struct Memory {
    memory: Box<[u8]>,
}

impl Memory {
    pub fn new(length: u32) -> Self {
        Memory {
            memory: vec![0; length as usize].into_boxed_slice(),
        }
    }
}

impl MmtApi for Memory {
    fn read(&mut self, addr: u32, mask: Mask) -> u32 {
        let addr = addr;
        let data = match mask {
            Mask::Byte => self.memory[addr as usize] as u32,
            Mask::Half => {
                let mut data = 0;
                for i in 0..2 {
                    data |= (self.memory[(addr + i) as usize] as u32) << (i * 8);
                }
                data
            }
            Mask::Word => {
                let mut data = 0;
                for i in 0..4 {
                    data |= (self.memory[(addr + i) as usize] as u32) << (i * 8);
                }
                data
            }
            Mask::None => {
                let mut data = 0;
                for i in 0..4 {
                    data |= (self.memory[(addr + i) as usize] as u32) << (i * 8);
                }
                data
            }
        };
        data
    }

    fn write(&mut self, addr: u32, data: u32, mask: Mask) {
        let addr = addr;
        match mask {
            Mask::Byte => self.memory[addr as usize] = data as u8,
            Mask::Half => {
                for i in 0..2 {
                    self.memory[(addr + i) as usize] = (data >> (i * 8)) as u8;
                }
            }
            Mask::Word => {
                for i in 0..4 {
                    self.memory[(addr + i) as usize] = (data >> (i * 8)) as u8;
                }
            }
            Mask::None => {
                for i in 0..4 {
                    self.memory[(addr + i) as usize] = (data >> (i * 8)) as u8;
                }
            }
        }
    }
}
