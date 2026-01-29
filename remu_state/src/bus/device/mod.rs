remu_macro::mod_flat!(uart_simple);

use crate::bus::{BusFault, parse_usize_allow_hex_underscore};

pub(crate) trait DeviceAccess: Send + Sync {
    fn name(&self) -> &str;
    fn size(&self) -> usize;
    fn read(&mut self, len: usize, offset: usize) -> Result<&[u8], BusFault>;
    fn write(&mut self, len: usize, offset: usize, data: &[u8]) -> Result<(), BusFault>;
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
