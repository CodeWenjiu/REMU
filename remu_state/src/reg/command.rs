use std::ops::Range;

use remu_fmt::parse_prefixed_uint;
use remu_types::Gpr;

fn parse_half_open_range_usize(s: &str) -> Result<Range<usize>, String> {
    // Support half-open ranges of the form: `start..end`
    // `start` and `end` use the same prefixed-integer syntax as elsewhere (0x/0o/0b/0d, '_' allowed).
    let s = s.trim();
    let (start_s, end_s) = s
        .split_once("..")
        .ok_or_else(|| "expected half-open range of form START..END".to_string())?;

    let start = parse_prefixed_uint::<usize>(start_s.trim()).map_err(|e| e.to_string())?;
    let end = parse_prefixed_uint::<usize>(end_s.trim()).map_err(|e| e.to_string())?;

    if start > end {
        return Err("invalid range: START must be <= END".to_string());
    }

    Ok(start..end)
}

#[derive(Debug, clap::Subcommand)]
pub enum RegCmd {
    /// Read With Specefic Width
    Read {
        /// Address to set
        #[arg()]
        index: Gpr,
    },

    /// Print Reg Values
    Print {
        /// Range to print (half-open: START..END)
        #[arg(value_parser = parse_half_open_range_usize, default_value = "0..32")]
        range: Range<usize>,
    },

    /// Write Reg Value
    Write {
        /// Address to set
        #[arg()]
        index: Gpr,

        /// Value to set
        #[arg(value_parser = parse_prefixed_uint::<u32>)]
        value: u32,
    },
}
