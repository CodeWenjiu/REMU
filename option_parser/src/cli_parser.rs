use clap::{builder::styling, Parser, value_parser};
use remu_utils::{Platform, DifftestRef};

use std::{error::Error, str::FromStr};

#[derive(Debug, Clone)]
pub struct CliBin {
    pub load_addr: u32,
    pub file_path: String,
}

impl FromStr for CliBin {
    type Err = Box<dyn Error + Send + Sync>;

    // Expects input in the form "load_addr:file_path", e.g. "0x80000000:foo.bin"
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.splitn(2, ':');
        let load_addr_str = parts.next().ok_or("Missing load_addr")?;
        let file_path = parts.next().ok_or("Missing file_path")?.to_string();
        let load_addr = u32::from_str_radix(load_addr_str.trim_start_matches("0x"), 16)?;

        Ok(CliBin {
            load_addr,
            file_path,
        })
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, styles = styling::Styles::styled()
.header(styling::AnsiColor::Green.on_default().bold())
.usage(styling::AnsiColor::Green.on_default().bold())
.literal(styling::AnsiColor::Blue.on_default().bold())
.placeholder(styling::AnsiColor::Cyan.on_default()))]
pub struct CLI {
    /// primary bin file path
    #[arg(long)]
    pub primary_bin: Option<String>,
    
    /// additional bin file path
    #[arg(long)]
    pub additional_bin: Option<CliBin>,

    /// Platform
    #[arg(short, long, default_value("rv32i-emu"), value_parser = value_parser!(Platform))]
    pub platform: Platform,

    /// Enable Batch mode
    #[arg(short, long)]
    pub batch: bool,

    /// Enable Log
    #[arg(short, long)]
    pub log: bool,

    /// differtest file path (Will Enable if provided)
    #[arg(short, long, value_parser = value_parser!(DifftestRef))]
    pub differtest: Option<DifftestRef>,
}
