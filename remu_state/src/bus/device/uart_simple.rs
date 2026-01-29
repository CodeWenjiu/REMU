use std::io::{self, Write};

use crate::bus::{BusFault, device::DeviceAccess};

pub struct SimpleUart;

impl SimpleUart {
    pub fn new() -> Self {
        Self
    }
}

impl DeviceAccess for SimpleUart {
    fn name(&self) -> &str {
        "uart_simple"
    }

    fn size(&self) -> usize {
        1
    }

    fn read(&mut self, len: usize, offset: usize) -> Result<&[u8], BusFault> {
        // 既然 size 是 1，Bus 逻辑保证了 offset 只能是 0
        if offset != 0 {
            return Err(BusFault::Unmapped { addr: offset });
        }

        if len != 1 {
            return Err(BusFault::UnsupportedAccessWidth(len));
        }

        Ok(&[0])
    }

    fn write(&mut self, len: usize, offset: usize, data: &[u8]) -> Result<(), BusFault> {
        if offset != 0 {
            return Err(BusFault::Unmapped { addr: offset });
        }

        if len != 1 {
            return Err(BusFault::UnsupportedAccessWidth(len));
        }

        let byte = data[0];

        let stdout = io::stdout();
        let mut handle = stdout.lock();

        handle.write_all(&[byte]).map_err(|_| BusFault::IoError)?;

        handle.flush().map_err(|_| BusFault::IoError)?;

        Ok(())
    }
}
