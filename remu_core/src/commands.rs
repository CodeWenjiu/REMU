use clap::{Subcommand, builder::styling};

#[derive(clap::Parser, Debug)]
#[command(author, version, about, styles = styling::Styles::styled()
.header(styling::AnsiColor::Green.on_default().bold())
.usage(styling::AnsiColor::Green.on_default().bold())
.literal(styling::AnsiColor::Blue.on_default().bold())
.placeholder(styling::AnsiColor::Cyan.on_default()))]
pub(crate) struct CommandParser {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    /// continue the emulator
    Continue,

    /// Times printf
    Times,
}
