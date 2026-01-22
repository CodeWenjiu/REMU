use crate::{bus::BusCmds, reg::RegCmds};

#[derive(Debug, clap::Subcommand)]
pub enum StateCmds {
    /// Reg Command
    Reg {
        #[command(subcommand)]
        subcmd: RegCmds,
    },

    /// Bus Command
    Bus {
        #[command(subcommand)]
        subcmd: BusCmds,
    },
}
