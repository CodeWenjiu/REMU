#[derive(Debug, clap::Subcommand)]
pub enum StateCmds {
    /// Hello Test
    Hello,

    /// Print Memory Contents
    Print {
        /// Address to start printing from
        start: u64,
        /// Number of bytes to print
        count: u64,
    },
}
