use clap::{command, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct CmdParser {
    #[command(subcommand)]
    command: Cmds,
}

use std::num::ParseIntError;
// hex parser
fn parse_hex(src: &str) -> Result<u32, ParseIntError> {
    if src.starts_with("0x") || src.starts_with("0X") {
        u32::from_str_radix(&src[2..], 16)
    } else {
        // 如果没有提供 0x 前缀，则尝试直接解析为十进制
        src.parse::<u32>()
    }
}

#[derive(Debug, Subcommand)]
#[command(author, version, about)]
pub enum Cmds {
    /// run single instrcution in the emulator
    #[clap(visible_alias = "si")]
    SingleInstrcution {
        count: Option<u64>,
    },

    /// continue the emulator
    #[clap(visible_alias = "c")]
    Continue {},

    /// Times printf
    #[clap(visible_alias = "t")]
    Times {},

    /// Memory examine
    #[clap(visible_alias = "x")]
    Examine {
        /// The target address(hex) and length
        #[arg(value_parser = parse_hex)]
        addr: u32,

        length: Option<u64>,
    },
}
