remu_macro::mod_flat!(uart_simple, uart16550, sifive_test_finisher);

use std::backtrace::Backtrace;

use crate::bus::{BusError, parse_usize_allow_hex_underscore};

pub(crate) trait DeviceAccess: Send + Sync {
    fn name(&self) -> &str;
    fn size(&self) -> usize;

    fn read_8(&mut self, offset: usize) -> Result<u8, BusError> {
        let _ = offset;
        Err(BusError::UnsupportedAccessWidth(8, Backtrace::capture()))
    }
    fn read_16(&mut self, offset: usize) -> Result<u16, BusError> {
        let _ = offset;
        Err(BusError::UnsupportedAccessWidth(16, Backtrace::capture()))
    }
    fn read_32(&mut self, offset: usize) -> Result<u32, BusError> {
        let _ = offset;
        Err(BusError::UnsupportedAccessWidth(32, Backtrace::capture()))
    }
    fn read_64(&mut self, offset: usize) -> Result<u64, BusError> {
        let _ = offset;
        Err(BusError::UnsupportedAccessWidth(64, Backtrace::capture()))
    }
    fn read_128(&mut self, offset: usize) -> Result<u128, BusError> {
        let _ = offset;
        Err(BusError::UnsupportedAccessWidth(128, Backtrace::capture()))
    }

    fn write_8(&mut self, offset: usize, value: u8) -> Result<(), BusError> {
        let _ = (offset, value);
        Err(BusError::UnsupportedAccessWidth(8, Backtrace::capture()))
    }
    fn write_16(&mut self, offset: usize, value: u16) -> Result<(), BusError> {
        let _ = (offset, value);
        Err(BusError::UnsupportedAccessWidth(16, Backtrace::capture()))
    }
    fn write_32(&mut self, offset: usize, value: u32) -> Result<(), BusError> {
        let _ = (offset, value);
        Err(BusError::UnsupportedAccessWidth(32, Backtrace::capture()))
    }
    fn write_64(&mut self, offset: usize, value: u64) -> Result<(), BusError> {
        let _ = (offset, value);
        Err(BusError::UnsupportedAccessWidth(64, Backtrace::capture()))
    }
    fn write_128(&mut self, offset: usize, value: u128) -> Result<(), BusError> {
        let _ = (offset, value);
        Err(BusError::UnsupportedAccessWidth(128, Backtrace::capture()))
    }
}

use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceConfig {
    pub name: String,
    pub start: usize,
}

impl FromStr for DeviceConfig {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let input = s.trim();
        if input.is_empty() {
            return Err("empty device spec".to_string());
        }

        let (name, start_str) = input.split_once('@').ok_or_else(|| {
            "invalid device spec: missing '@' (expected <name>@<start>)".to_string()
        })?;

        let name = name.trim();
        if name.is_empty() {
            return Err("invalid device spec: empty name before '@'".to_string());
        }

        let start = parse_usize_allow_hex_underscore(start_str, "device address")?;

        Ok(DeviceConfig {
            name: name.to_string(),
            start,
        })
    }
}

pub(crate) fn get_device(name: &str) -> Option<Box<dyn DeviceAccess>> {
    match name {
        "uart_simple" => Some(Box::new(uart_simple::SimpleUart::new())),
        "uart16550" => Some(Box::new(uart16550::Uart16550::new())),
        "sifive_test_finisher" => Some(Box::new(sifive_test_finisher::SifiveTestFinisher::new())),
        _ => None,
    }
}
