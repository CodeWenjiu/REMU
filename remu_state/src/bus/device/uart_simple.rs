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

    fn read_8(&mut self, offset: usize) -> Result<u8, BusFault> {
        let _ = offset;
        Ok(0)
    }

    fn write_8(&mut self, offset: usize, value: u8) -> Result<(), BusFault> {
        let _ = offset;

        let stdout = io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(&[value]).map_err(|_| BusFault::IoError)?;
        handle.flush().map_err(|_| BusFault::IoError)?;
        Ok(())
    }
}
