use std::path::PathBuf;

use clap::ValueHint;

use crate::bus::{MemRegionSpec, device::DeviceConfig};

#[derive(clap::Args, Debug, Clone)]
pub struct BusOption {
    #[arg(
        long = "mem",
        value_name = "NAME@START:END",
        action = clap::ArgAction::Append,
        default_value = "ram@0x8000_0000:0x8800_0000"
    )]
    pub mem: Vec<MemRegionSpec>,

    #[arg(
        long = "dev",
        value_name = "NAME@START",
        action = clap::ArgAction::Append,
        default_values = ["uart16550@0x1000_0000", "sifive_test_finisher@0x0010_0000"]
    )]
    pub devices: Vec<DeviceConfig>,

    #[arg(long = "elf", alias = "bin", value_name = "PATH", value_parser = file_exists, value_hint = ValueHint::FilePath)]
    pub elf: Option<PathBuf>,
}

fn file_exists(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if path.exists() && path.is_file() {
        Ok(path)
    } else {
        Err(format!("File Does Not Exist or It is Not a File: '{}'", s))
    }
}
