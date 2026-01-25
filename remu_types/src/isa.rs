use std::str::FromStr;

use target_lexicon::{Architecture, Triple};

use crate::Xlen;

#[derive(Debug, Clone)]
pub struct IsaSpec(pub Architecture);

impl FromStr for IsaSpec {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized_s = if s.contains('-') {
            s.to_string()
        } else {
            format!("{}-unkonwn-none-elf", s)
        };

        let architecture = normalized_s
            .parse::<Triple>()
            .map_err(|e| format!("Invalid ISA string: '{}',: {}", s, e))?
            .architecture;

        match architecture {
            Architecture::Riscv32(_) | Architecture::Riscv64(_) => Ok(IsaSpec(architecture)),
            _ => Err(format!("Unsupported ISA architecture: {}", architecture)),
        }
    }
}

pub trait RvIsa: 'static + Copy {
    type XLEN: Xlen;
    const HAS_M: bool;
}

#[derive(Debug, Clone, Copy)]
pub struct Rv32<const M: bool>;

impl<const M: bool> RvIsa for Rv32<M> {
    type XLEN = u32;
    const HAS_M: bool = M;
}
