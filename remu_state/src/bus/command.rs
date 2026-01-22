use remu_fmt::{parse_byte_vec, parse_prefixed_uint};

#[derive(Debug, clap::Subcommand)]
pub enum BusCmd {
    /// Read With Specefic Width
    Read {
        #[command(subcommand)]
        subcmd: ReadCommand,
    },

    /// Print Memory Contents
    Print {
        /// Address to start printing from (e.g. `0x1000`, `0o377`, `0b1010`, `1234`, `0d1234`)
        #[arg(value_parser = parse_prefixed_uint::<usize>)]
        addr: usize,

        /// Number of bytes to print (e.g. `16`, `0x10`)
        #[arg(value_parser = parse_prefixed_uint::<usize>)]
        count: usize,
    },

    /// Write Memory Value
    Write {
        #[command(subcommand)]
        subcmd: WriteCommand,
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

#[derive(Debug, clap::Subcommand)]
pub enum ReadCommand {
    U8(ReadArgs),
    U16(ReadArgs),
    U32(ReadArgs),
    U64(ReadArgs),
    U128(ReadArgs),
}

#[derive(Debug, clap::Args)]
pub struct ReadArgs {
    /// Address to start read
    #[arg(value_parser = parse_prefixed_uint::<usize>)]
    pub addr: usize,
}

#[derive(Debug, clap::Subcommand)]
pub enum WriteCommand {
    U8 {
        /// Address to start write
        #[arg(value_parser = parse_prefixed_uint::<usize>)]
        addr: usize,

        /// Value to write
        #[arg(value_parser = parse_prefixed_uint::<u8>)]
        value: u8,
    },

    U16 {
        /// Address to start write
        #[arg(value_parser = parse_prefixed_uint::<usize>)]
        addr: usize,

        /// Value to write
        #[arg(value_parser = parse_prefixed_uint::<u16>)]
        value: u16,
    },

    U32 {
        /// Address to start write
        #[arg(value_parser = parse_prefixed_uint::<usize>)]
        addr: usize,

        /// Value to write
        #[arg(value_parser = parse_prefixed_uint::<u32>)]
        value: u32,
    },

    U64 {
        /// Address to start write
        #[arg(value_parser = parse_prefixed_uint::<usize>)]
        addr: usize,

        /// Value to write
        #[arg(value_parser = parse_prefixed_uint::<u64>)]
        value: u64,
    },

    U128 {
        /// Address to start write
        #[arg(value_parser = parse_prefixed_uint::<usize>)]
        addr: usize,

        /// Value to write
        #[arg(value_parser = parse_prefixed_uint::<u128>)]
        value: u128,
    },
}
