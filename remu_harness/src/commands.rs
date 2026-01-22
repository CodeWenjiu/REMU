use clap::builder::styling;
use remu_simulator::SimulatorOption;
use remu_state::StateCmds;

#[derive(clap::Args, Debug, Clone)]
pub struct HarnessOption {
    /// Simulator Option
    #[command(flatten)]
    pub simulator: SimulatorOption,
}

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
    pub command: Commands,
}

#[derive(Debug, clap::Subcommand)]
pub enum Commands {
    /// continue the emulator
    Continue,

    /// Times printf
    Times {
        #[command(subcommand)]
        subcmd: TimeCmds,
    },

    /// State Command
    State {
        #[command(subcommand)]
        subcmd: StateCmds,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum TimeCmds {
    /// Times Count
    Count {
        #[command(subcommand)]
        subcmd: TimeCountCmds,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum TimeCountCmds {
    Test,
}
