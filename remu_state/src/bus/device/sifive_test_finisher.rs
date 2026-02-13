use remu_types::ExitCode;

use crate::bus::{BusError, device::DeviceAccess};

pub struct SifiveTestFinisher;

impl SifiveTestFinisher {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SifiveTestFinisher {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceAccess for SifiveTestFinisher {
    fn name(&self) -> &str {
        "sifive_test_finisher"
    }

    fn size(&self) -> usize {
        4
    }

    fn read_32(&mut self, offset: usize) -> Result<u32, BusError> {
        let _ = offset;
        Ok(0)
    }

    fn write_32(&mut self, offset: usize, value: u32) -> Result<(), BusError> {
        let _ = offset;
        let exit_code = match value {
            0x5555 => ExitCode::Good,
            _ => ExitCode::Bad, // 0x3333 (fail) or other
        };
        Err(BusError::ProgramExit(exit_code))
    }
}
