remu_macro::mod_pub!(reg, extension, extension_enum, extension_v);

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

    /// V extension options: FP level, ELEN, VLENB, and VectorCsrState. Use [`NoV`](crate::isa::extension_v::NoV) when disabled.
    type VConfig: crate::isa::extension_v::VExtensionConfig;

    const ISA_STR: &'static str = "rv32i";

    /// Read-only MISA value (XLEN + extensions). Used for CSR read and difftest.
    const MISA: u32 = 0x4000_0100; // rv32i default

    const HAS_M: bool = <Self::Conf as ArchConfig>::M::ENABLED;
    const HAS_F: bool = <Self::Conf as ArchConfig>::F::ENABLED;

    /// CSRs to compare in difftest, as segments: base segment(s) + optional extension segment(s).
    /// Default: base only. Override when V is present (e.g. [`CSRS_FOR_DIFFTEST_V`](crate::isa::reg::CSRS_FOR_DIFFTEST_V)).
    fn csrs_for_difftest() -> &'static [&'static [crate::isa::reg::Csr]]
    where
        Self: Sized,
    {
        &[CSRS_FOR_DIFFTEST_BASE]
    }
}

/// Extension suffix (parsed from the part after the first `_` in the ISA string).
/// Add new variants when supporting more extension combinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtensionSpec {
    #[default]
    None,
    /// Zve32x + Zvl128b (V extension, VLENB=16).
    Zve32xZvl128b,
}

impl FromStr for ExtensionSpec {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(ExtensionSpec::None);
        }
        match to_ascii_lowercase(s).as_str() {
            "zve32x_zvl128b" => Ok(ExtensionSpec::Zve32xZvl128b),
            _ => Err(format!("Unrecognized extension spec: '{}'", s)),
        }
    }
}

/// ISA selector: base architecture (via target_lexicon Triple) + optional extension spec.
/// Parse with first `_` as separator: prefix → Triple, suffix → ExtensionSpec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsaSpec {
    /// Base architecture from Triple (e.g. riscv32i, riscv32im).
    pub base: Architecture,
    /// Optional extensions (parsed from substring after first `_`).
    pub extensions: ExtensionSpec,
}

impl IsaSpec {
    /// Architecture for disassembly (ByteGuesser, etc.). Uses the Triple base.
    #[inline]
    pub fn architecture(self) -> Architecture {
        self.base
    }
}

impl FromStr for IsaSpec {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (prefix, suffix) = match s.split_once('_') {
            Some((p, suf)) => (p.trim(), suf.trim()),
            None => (s, ""),
        };

        let normalized = if prefix.contains('-') {
            prefix.to_string()
        } else {
            format!("{}-unknown-none-elf", prefix)
        };

        let base = normalized
            .parse::<Triple>()
            .map_err(|e| format!("Invalid ISA string: '{}',: {}", s, e))?
            .architecture;

        let architecture = match base {
            Architecture::Riscv32(_) | Architecture::Riscv64(_) => base,
            _ => return Err(format!("Unsupported ISA architecture: {}", base)),
        };

        let extensions = ExtensionSpec::from_str(suffix)?;

        Ok(IsaSpec { base: architecture, extensions })
    }
}

fn to_ascii_lowercase(s: &str) -> String {
    s.chars().map(|c| c.to_ascii_lowercase()).collect()
}
