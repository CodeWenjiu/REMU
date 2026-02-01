remu_macro::mod_flat!(uart_simple);

use crate::bus::{BusFault, parse_usize_allow_hex_underscore};

pub(crate) trait DeviceAccess: Send + Sync {
    fn name(&self) -> &str;
    fn size(&self) -> usize;

    fn read_8(&mut self, offset: usize) -> Result<u8, BusFault> {
        let _ = offset;
        Err(BusFault::UnsupportedAccessWidth(8))
    }
    fn read_16(&mut self, offset: usize) -> Result<u16, BusFault> {
        let _ = offset;
        Err(BusFault::UnsupportedAccessWidth(16))
    }
    fn read_32(&mut self, offset: usize) -> Result<u32, BusFault> {
        let _ = offset;
        Err(BusFault::UnsupportedAccessWidth(32))
    }
    fn read_64(&mut self, offset: usize) -> Result<u64, BusFault> {
        let _ = offset;
        Err(BusFault::UnsupportedAccessWidth(64))
    }
    fn read_128(&mut self, offset: usize) -> Result<u128, BusFault> {
        let _ = offset;
        Err(BusFault::UnsupportedAccessWidth(128))
    }

    fn write_8(&mut self, offset: usize, value: u8) -> Result<(), BusFault> {
        let _ = (offset, value);
        Err(BusFault::UnsupportedAccessWidth(8))
    }
    fn write_16(&mut self, offset: usize, value: u16) -> Result<(), BusFault> {
        let _ = (offset, value);
        Err(BusFault::UnsupportedAccessWidth(16))
    }
    fn write_32(&mut self, offset: usize, value: u32) -> Result<(), BusFault> {
        let _ = (offset, value);
        Err(BusFault::UnsupportedAccessWidth(32))
    }
    fn write_64(&mut self, offset: usize, value: u64) -> Result<(), BusFault> {
        let _ = (offset, value);
        Err(BusFault::UnsupportedAccessWidth(64))
    }
    fn write_128(&mut self, offset: usize, value: u128) -> Result<(), BusFault> {
        let _ = (offset, value);
        Err(BusFault::UnsupportedAccessWidth(128))
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
        _ => None,
    }
}
