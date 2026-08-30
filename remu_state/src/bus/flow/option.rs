use std::path::PathBuf;

use clap::ValueHint;

use crate::bus::{MemRegionSpec, device::DeviceConfig};

#[derive(clap::Args, Debug, Clone)]
pub struct BusOption {
    /// Base memory config file (one `NAME@START:END` per line). If omitted, a
    /// built-in default (`ram@0x8000_0000:0x8800_0000`) is used.
    #[arg(long = "mem-base", value_name = "FILE", value_hint = ValueHint::FilePath)]
    pub mem_base: Option<PathBuf>,

    /// Base device config file (one `KIND@START` per line). If omitted, a
    /// built-in default (uart16550, sifive_test_finisher, clint) is used.
    #[arg(long = "dev-base", value_name = "FILE", value_hint = ValueHint::FilePath)]
    pub dev_base: Option<PathBuf>,

    /// Extra memory regions appended to the base set.
    #[arg(
        long = "mem",
        value_name = "NAME@START:END",
        action = clap::ArgAction::Append
    )]
    pub mem: Vec<MemRegionSpec>,

    /// Extra devices appended to the base set.
    #[arg(
        long = "dev",
        value_name = "KIND@START",
        action = clap::ArgAction::Append
    )]
    pub devices: Vec<DeviceConfig>,

    #[arg(long = "elf", alias = "bin", value_name = "PATH", value_parser = file_exists, value_hint = ValueHint::FilePath)]
    pub elf: Option<PathBuf>,

    /// Application arguments written to 0x87FF_F000 before boot.
    #[arg(long = "app-args", value_name = "ARGS")]
    pub app_args: Option<String>,
}

impl BusOption {
    /// Resolve the full memory region list: base (file or built-in default)
    /// followed by the `--mem` extras.
    pub fn resolve_mem_regions(&self) -> Vec<MemRegionSpec> {
        let mut base = match &self.mem_base {
            Some(path) => read_specs::<MemRegionSpec>(path),
            None => default_mem_regions(),
        };
        base.extend(self.mem.iter().cloned());
        base
    }

    /// Resolve the full device list: base (file or built-in default) followed by
    /// the `--dev` extras.
    pub fn resolve_devices(&self) -> Vec<DeviceConfig> {
        let mut base = match &self.dev_base {
            Some(path) => read_specs::<DeviceConfig>(path),
            None => default_devices(),
        };
        base.extend(self.devices.iter().cloned());
        base
    }
}

/// Read a config file with one spec per line (blank/`#` lines ignored).
fn read_specs<T: std::str::FromStr<Err = String>>(path: &PathBuf) -> Vec<T> {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read config file {}: {e}", path.display()));
    content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| T::from_str(l).unwrap_or_else(|e| panic!("bad spec in {}: {e}", path.display())))
        .collect()
}

/// Built-in default memory regions (used when `--mem-base` is omitted).
fn default_mem_regions() -> Vec<MemRegionSpec> {
    vec![
        "ram@0x8000_0000:0x8800_0000"
            .parse()
            .expect("built-in default mem"),
    ]
}

/// Built-in default devices (used when `--dev-base` is omitted).
fn default_devices() -> Vec<DeviceConfig> {
    [
        "uart16550@0x1000_0000",
        "sifive_test_finisher@0x0010_0000",
        "clint@0x0200_0000",
    ]
    .iter()
    .map(|s| s.parse().expect("built-in default dev"))
    .collect()
}

fn file_exists(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if path.exists() && path.is_file() {
        Ok(path)
    } else {
        Err(format!("File Does Not Exist or It is Not a File: '{}'", s))
    }
}
