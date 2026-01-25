use clap::builder::styling;
use remu_state::StateCmd;

use crate::FuncCmd;

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
pub struct SimulatorCommand {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    /// continue the emulator
    Continue,

    /// Step
    Step {
        /// Number of steps to take
        #[arg(default_value_t = 1)]
        times: usize,
    },

    /// State Command
    State {
        #[command(subcommand)]
        subcmd: StateCmd,
    },

    /// Function Command
    Func {
        #[command(subcommand)]
        subcmd: FuncCmd,
    },
}
