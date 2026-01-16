use remu_simulator::SimulatorOption;

#[derive(clap::Args, Debug)]
pub struct HarnessOption {
    /// Simulator Option
    #[command(flatten)]
    pub simulator: SimulatorOption,
}

#[derive(Debug, clap::Subcommand)]
pub enum HarnessCommands {
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

#[derive(Debug, clap::Subcommand)]
pub enum StateCmds {
    /// Hello Test
    Hello,
}
