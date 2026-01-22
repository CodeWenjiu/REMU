use clap::builder::styling;
use remu_state::StateCmd;

#[derive(clap::Parser, Debug)]
#[command(
    author,
    version,
    about,
    disable_help_flag = true,
    disable_version_flag = true,
    styles = styling::Styles::styled()
    .header(styling::AnsiColor::Green.on_default().bold())
    .usage(styling::AnsiColor::Green.on_default().bold())
    .literal(styling::AnsiColor::Blue.on_default().bold())
    .placeholder(styling::AnsiColor::Cyan.on_default())
)]
pub struct CommandParser {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    /// continue the emulator
    Continue,

    /// Times printf
    Times {
        #[command(subcommand)]
        subcmd: TimeCmd,
    },

    /// State Command
    State {
        #[command(subcommand)]
        subcmd: StateCmd,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum TimeCmd {
    /// Times Count
    Count {
        #[command(subcommand)]
        subcmd: TimeCountCmd,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum TimeCountCmd {
    Test,
}
