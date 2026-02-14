use std::ops::Range;

use remu_fmt::{parse_byte_vec, parse_prefixed_uint};
use remu_types::isa::reg::{Csr as CsrReg, Fpr, Gpr};

fn parse_vr_index(s: &str) -> Result<usize, String> {
    let i = parse_prefixed_uint::<usize>(s.trim()).map_err(|e| e.to_string())?;
    if i < 32 {
        Ok(i)
    } else {
        Err(format!("VR index must be 0..32, got {}", i))
    }
}

fn parse_half_open_range_usize(s: &str) -> Result<Range<usize>, String> {
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
    /// General Purpose Registers (x0–x31 / ra, sp, …)
    Gpr {
        #[command(subcommand)]
        subcmd: GprRegCmd,
    },

    /// Floating-point Registers (f0–f31 / ft0, fa0, fs0 等 ABI 名)
    Fpr {
        #[command(subcommand)]
        subcmd: FprRegCmd,
    },

    /// Program Counter pc
    Pc {
        #[command(subcommand)]
        subcmd: PcRegCmd,
    },

    /// Vector Registers (v0–v31). No-op when V extension is disabled.
    Vr {
        #[command(subcommand)]
        subcmd: VrRegCmd,
    },

    /// Control and Status Registers (mstatus, mepc, …)
    Csr {
        #[command(subcommand)]
        subcmd: CsrRegCmd,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum GprRegCmd {
    Read {
        #[arg()]
        index: Gpr,
    },

    Print {
        #[arg(value_parser = parse_half_open_range_usize, default_value = "0..32")]
        range: Range<usize>,
    },

    Write {
        #[arg()]
        index: Gpr,

        #[arg(value_parser = parse_prefixed_uint::<u32>)]
        value: u32,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum FprRegCmd {
    Read {
        #[arg()]
        index: Fpr,
    },

    Print {
        #[arg(value_parser = parse_half_open_range_usize, default_value = "0..32")]
        range: Range<usize>,
    },

    /// 写单个 FPR（可用 ABI 名如 fa0 或 f10）
    Write {
        #[arg()]
        index: Fpr,

        #[arg(value_parser = parse_prefixed_uint::<u32>)]
        value: u32,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum VrRegCmd {
    Read {
        #[arg(value_parser = parse_vr_index)]
        index: usize,
    },

    Print {
        #[arg(value_parser = parse_half_open_range_usize, default_value = "0..32")]
        range: Range<usize>,
    },

    Write {
        #[arg(value_parser = parse_vr_index)]
        index: usize,

        /// Bytes in hex (e.g. `0xdeadbeef`) or quoted string; length is padded/truncated to VLENB.
        #[arg(value_parser = parse_byte_vec)]
        value: Vec<u8>,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum CsrRegCmd {
    Read {
        #[arg()]
        index: CsrReg,
    },

    Write {
        #[arg()]
        index: CsrReg,

        #[arg(value_parser = parse_prefixed_uint::<u32>)]
        value: u32,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum PcRegCmd {
    Read,

    Write {
        #[arg(value_parser = parse_prefixed_uint::<u32>)]
        value: u32,
    },
}
