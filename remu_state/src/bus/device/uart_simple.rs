use std::io::{self, Write};

use crate::bus::{BusError, device::DeviceAccess};

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

    fn read_8(&mut self, offset: usize) -> Result<u8, BusError> {
        let _ = offset;
        Ok(0)
    }

    fn write_8(&mut self, offset: usize, value: u8) -> Result<(), BusError> {
        let _ = offset;

        let stdout = io::stdout();
        let mut handle = stdout.lock();
        handle
            .write_all(&[value])
            .map_err(|_| BusError::IoError(format!("{}", std::backtrace::Backtrace::capture())))?;
        handle
            .flush()
            .map_err(|_| BusError::IoError(format!("{}", std::backtrace::Backtrace::capture())))?;
        Ok(())
    }
}
