use remu_fmt::{parse_byte_vec, parse_prefixed_uint};

#[derive(Debug, clap::Subcommand)]
pub enum StateCmds {
    /// Hello Test
    Hello,

    /// Print Memory Contents
    Print {
        /// Address to start printing from (e.g. `0x1000`, `0o377`, `0b1010`, `1234`, `0d1234`)
        #[arg(value_parser = parse_prefixed_uint::<usize>)]
        start: usize,

        /// Number of bytes to print (e.g. `16`, `0x10`)
        #[arg(value_parser = parse_prefixed_uint::<usize>)]
        count: usize,
    },

    /// Set Memory Value
    Set {
        /// Address to set
        #[arg(value_parser = parse_prefixed_uint::<usize>)]
        address: usize,
        /// Value to set (e.g. `0xdead_beef` or `[0xde, 0xad, 0xbe, 0xef]` or `[0xdead, 0xbe, 0xef]`)
        #[arg(value_parser = parse_byte_vec)]
        value: Vec<Vec<u8>>,
    },
}
