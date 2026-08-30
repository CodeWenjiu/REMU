remu_macro::mod_prv!(uart_simple, uart16550, sifive_test_finisher, clint, display);

use std::backtrace::Backtrace;
use std::str::FromStr;

use crate::bus::memory::MemRegionSpec;
use crate::bus::{BusError, parse_usize_allow_hex_underscore};

pub(crate) trait DeviceAccess: Send + Sync {
    fn name(&self) -> &str;
    fn size(&self) -> usize;

    /// Extra memory regions this device requires (e.g. a framebuffer). These are
    /// appended to the bus memory map before `Bus::new` builds `Memory`.
    /// Default: none.
    fn extra_mem_regions(&self) -> Vec<MemRegionSpec> {
        Vec::new()
    }

    /// Attach a memory region pointer to this device (called after `Memory` is
    /// built, for devices whose `extra_mem_regions` were allocated). Default: no-op.
    fn attach_mem_region(&mut self, _base: usize, _ptr: *mut u8, _size: usize) {}

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

/// MMIO device kind: fixed set, matches [`instantiate_device`].
/// Parsed from `--dev` before `@` (snake_case strings); clap uses [`DeviceConfig`]'s [`FromStr`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceKind {
    UartSimple,
    Uart16550,
    Clint,
    SifiveTestFinisher,
    Display,
}

impl DeviceKind {
    #[inline]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UartSimple => "uart_simple",
            Self::Uart16550 => "uart16550",
            Self::Clint => "clint",
            Self::SifiveTestFinisher => "sifive_test_finisher",
            Self::Display => "display",
        }
    }
}

impl FromStr for DeviceKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "uart_simple" => Ok(Self::UartSimple),
            "uart16550" => Ok(Self::Uart16550),
            "clint" => Ok(Self::Clint),
            "sifive_test_finisher" => Ok(Self::SifiveTestFinisher),
            "display" => Ok(Self::Display),
            _ => Err(format!(
                "unknown device kind {s:?}; expected uart_simple, uart16550, clint, sifive_test_finisher, display"
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceConfig {
    pub kind: DeviceKind,
    pub start: usize,
}

impl FromStr for DeviceConfig {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let input = s.trim();
        if input.is_empty() {
            return Err("empty device spec".to_string());
        }

        let (kind_str, start_str) = input.split_once('@').ok_or_else(|| {
            "invalid device spec: missing '@' (expected <kind>@<start>)".to_string()
        })?;

        let kind = DeviceKind::from_str(kind_str)?;
        let start = parse_usize_allow_hex_underscore(start_str, "device address")?;

        Ok(DeviceConfig { kind, start })
    }
}

pub(crate) fn instantiate_device(kind: DeviceKind) -> Box<dyn DeviceAccess> {
    match kind {
        DeviceKind::UartSimple => Box::new(uart_simple::SimpleUart::new()),
        DeviceKind::Uart16550 => Box::new(uart16550::Uart16550::new()),
        DeviceKind::SifiveTestFinisher => Box::new(sifive_test_finisher::SifiveTestFinisher::new()),
        DeviceKind::Clint => Box::new(clint::Clint::new()),
        DeviceKind::Display => Box::new(display::Display::new()),
    }
}
