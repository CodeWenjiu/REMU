use remu_fmt::parse_prefixed_uint;

#[derive(Debug, clap::Subcommand)]
pub enum RegCmd {
    /// Read With Specefic Width
    Read {
        /// Address to set
        #[arg()]
        index: usize,
    },

    /// Write Reg Value
    Write {
        /// Address to set
        #[arg()]
        index: usize,

        /// Value to set
        #[arg(value_parser = parse_prefixed_uint::<u32>)]
        value: u32,
    },
}
