remu_macro::mod_pub!(reg, extension, extension_enum);

use std::str::FromStr;

use core::ops::{Deref, DerefMut, Index};
use target_lexicon::{Architecture, Triple};

use crate::{
    Xlen,
    isa::{extension::Extension, reg::CSRS_FOR_DIFFTEST_BASE},
};

pub trait ArchConfig: 'static + Copy {
    type M: Extension<State = ()>;

    type F: Extension;
}

pub trait RvIsa: 'static + Copy {
    type XLEN: Xlen;
    type Conf: ArchConfig;

    type PcState: Default
        + Copy
        + PartialEq
        + std::fmt::Debug
        + crate::isa::reg::RegDiff
        + From<u32>
        + Deref<Target = u32>
        + DerefMut;
    type GprState: Default
        + Copy
        + PartialEq
        + std::fmt::Debug
        + crate::isa::reg::RegAccess<Item = u32>
        + crate::isa::reg::RegDiff
        + Index<usize, Output = u32>;
    type FprState: Default
        + Copy
        + PartialEq
        + std::fmt::Debug
        + crate::isa::reg::FprAccess
        + crate::isa::reg::RegDiff;

    /// Vector CSR state: `()` when no V extension, [`VectorCsrFields<VLENB>`](crate::isa::reg::VectorCsrFields) when V is present.
    type VectorCsrState: crate::isa::reg::VectorCsrState;

    const ISA_STR: &'static str = "rv32i";

    /// Read-only MISA value (XLEN + extensions). Used for CSR read and difftest.
    const MISA: u32 = 0x4000_0100; // rv32i default

    const HAS_M: bool = <Self::Conf as ArchConfig>::M::ENABLED;
    const HAS_F: bool = <Self::Conf as ArchConfig>::F::ENABLED;

    /// Whether the V (vector) extension is present. When false, vector CSRs are absent (read 0, write no-op).
    const HAS_V: bool = false;
    /// VLEN/8 in bytes; only meaningful when HAS_V. Used as the return value of the vlenb CSR.
    const VLENB: u32 = 0;

    /// CSRs to compare in difftest, as segments: base segment(s) + optional extension segment(s).
    /// Default: base only. Override to add e.g. [`CSRS_FOR_DIFFTEST_V`](crate::isa::reg::CSRS_FOR_DIFFTEST_V) when HAS_V.
    fn csrs_for_difftest() -> &'static [&'static [crate::isa::reg::Csr]]
    where
        Self: Sized,
    {
        &[CSRS_FOR_DIFFTEST_BASE]
    }
}

#[derive(Debug, Clone)]
pub struct IsaSpec(pub Architecture);

impl FromStr for IsaSpec {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized_s = if s.contains('-') {
            s.to_string()
        } else {
            format!("{}-unknown-none-elf", s)
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
