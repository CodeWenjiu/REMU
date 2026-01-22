use crate::{bus::BusCmd, reg::RegCmd};

#[derive(Debug, clap::Subcommand)]
pub enum StateCmd {
    /// Reg Command
    Reg {
        #[command(subcommand)]
        subcmd: RegCmd,
    },

    /// Bus Command
    Bus {
        #[command(subcommand)]
        subcmd: BusCmd,
    },

    /// Show current memory map (name | begin | end)
    MemMap,
}
