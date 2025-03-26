use clap::{builder::styling, Parser, value_parser};
use remu_utils::{Platform, DifftestRef};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, styles = styling::Styles::styled()
.header(styling::AnsiColor::Green.on_default().bold())
.usage(styling::AnsiColor::Green.on_default().bold())
.literal(styling::AnsiColor::Blue.on_default().bold())
.placeholder(styling::AnsiColor::Cyan.on_default()))]
pub struct CLI {
    /// bin file path
    #[arg(long)]
    pub bin: Option<String>,

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
