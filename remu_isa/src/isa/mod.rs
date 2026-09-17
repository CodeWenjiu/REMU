remu_macro::mod_pub!(reg, extension, extension_enum, extension_v, isa_kind);

pub use isa_kind::IsaKind;

use std::str::FromStr;

use core::ops::{Deref, DerefMut, Index};

use crate::Xlen;
use crate::isa::reg::CSRS_FOR_DIFFTEST_BASE;

pub trait RvIsa: 'static + Copy {
    type XLEN: Xlen;

    type PcState: Default
        + Copy
        + PartialEq
        + std::fmt::Debug
        + crate::isa::reg::RegDiff
        + From<Self::XLEN>
        + Deref<Target = Self::XLEN>
        + DerefMut;
    type GprState: Default
        + Copy
        + PartialEq
        + std::fmt::Debug
        + crate::isa::reg::RegAccess<Item = Self::XLEN>
        + crate::isa::reg::RegDiff
        + Index<usize, Output = Self::XLEN>;
    type FprState: Default
        + Copy
        + PartialEq
        + std::fmt::Debug
        + crate::isa::reg::FprAccess
        + crate::isa::reg::RegDiff;

    /// V extension options. Use [`NoV`](crate::isa::extension_v::NoV) when disabled.
    type VConfig: crate::isa::extension_v::VExtensionConfig;

    const ISA_STR: &'static str = "rv32i";
    /// Low 32 bits of the `misa` CSR. The MXL field lives in bits [63:62] and
    /// is derived from [`XLEN`](Self::XLEN) (32 -> 01, 64 -> 10), never stored
    /// here — RV64's MXL does not fit in the low word.
    const MISA: u32 = 0x4000_0100;
    const HAS_M: bool = false;
    const HAS_F: bool = false;
    const HAS_WJ_CUS0: bool = false;

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
    /// **wjCus0** custom opcode set (MNIST / accelerator); not a standard RISC-V letter.
    /// ISA strings: `riscv32i_wjCus0`, `riscv32im_wjCus0`.
    WjCus0,
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
            "wjcus0" => Ok(ExtensionSpec::WjCus0),
            _ => Err(format!("Unrecognized extension spec: '{}'", s)),
        }
    }
}

/// Base ISA (XLEN + base extension letters), parsed directly from the ISA string.
///
/// We intentionally do **not** use target-lexicon for this: it has no
/// `riscv64i`/`riscv64im` variants (only gc/imac) and its vendor/os/abi triple
/// concept is meaningless for a bare-metal simulator. Add new variants here when
/// supporting more base ISAs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsaBase {
    /// riscv32i
    Rv32I,
    /// riscv32im
    Rv32Im,
    /// riscv64i
    Rv64I,
    /// riscv64im
    Rv64Im,
}

impl IsaBase {
    /// XLEN in bits (32 or 64).
    #[inline]
    pub const fn xlen(self) -> u8 {
        match self {
            Self::Rv32I | Self::Rv32Im => 32,
            Self::Rv64I | Self::Rv64Im => 64,
        }
    }
}

/// ISA selector: base ISA + optional extension spec. Parse with first `_` as
/// separator: prefix → [`IsaBase`], suffix → [`ExtensionSpec`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsaSpec {
    /// Base ISA (e.g. riscv32im).
    pub base: IsaBase,
    /// Optional extensions (parsed from substring after first `_`).
    pub extensions: ExtensionSpec,
}

impl FromStr for IsaSpec {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (prefix, suffix) = match s.split_once('_') {
            Some((p, suf)) => (p.trim(), suf.trim()),
            None => (s, ""),
        };

        // Accept both shorthand (`riscv32im`) and full-triple forms
        // (`riscv32im-unknown-none-elf`); the vendor/os/abi part is ignored for
        // bare-metal simulation, exactly like before.
        let arch = prefix.split('-').next().unwrap_or(prefix);
        let base = match to_ascii_lowercase(arch).as_str() {
            "riscv32i" => IsaBase::Rv32I,
            "riscv32im" => IsaBase::Rv32Im,
            "riscv64i" => IsaBase::Rv64I,
            "riscv64im" => IsaBase::Rv64Im,
            other => {
                return Err(format!(
                    "unsupported base ISA '{other}'; supported: riscv32i, riscv32im, riscv64i, riscv64im"
                ));
            }
        };

        let extensions = ExtensionSpec::from_str(suffix)?;

        Ok(IsaSpec { base, extensions })
    }
}

fn to_ascii_lowercase(s: &str) -> String {
    s.chars().map(|c| c.to_ascii_lowercase()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(s: &str) -> IsaSpec {
        IsaSpec::from_str(s).unwrap()
    }

    #[test]
    fn parses_rv32_bases() {
        assert_eq!(parsed("riscv32i").base, IsaBase::Rv32I);
        assert_eq!(parsed("riscv32im").base, IsaBase::Rv32Im);
    }

    #[test]
    fn parses_rv64_bases() {
        assert_eq!(parsed("riscv64i").base, IsaBase::Rv64I);
        assert_eq!(parsed("riscv64im").base, IsaBase::Rv64Im);
    }

    #[test]
    fn parses_full_triple_form() {
        // vendor/os/abi part is ignored for bare-metal simulation
        assert_eq!(parsed("riscv64im-unknown-none-elf").base, IsaBase::Rv64Im);
        assert_eq!(parsed("riscv32im-unknown-none-elf").base, IsaBase::Rv32Im);
    }

    #[test]
    fn names_extension_suffix() {
        let spec = parsed("riscv32im_wjCus0");
        assert_eq!(spec.base, IsaBase::Rv32Im);
        assert_eq!(spec.extensions, ExtensionSpec::WjCus0);

        let spec = parsed("riscv32i_zve32x_zvl128b");
        assert_eq!(spec.base, IsaBase::Rv32I);
        assert_eq!(spec.extensions, ExtensionSpec::Zve32xZvl128b);
    }

    #[test]
    fn xlen_matches_base() {
        assert_eq!(IsaBase::Rv32I.xlen(), 32);
        assert_eq!(IsaBase::Rv32Im.xlen(), 32);
        assert_eq!(IsaBase::Rv64I.xlen(), 64);
        assert_eq!(IsaBase::Rv64Im.xlen(), 64);
    }

    #[test]
    fn rejects_unknown_or_unsupported_bases() {
        for s in [
            "riscv32imac",
            "riscv32",
            "riscv64",
            "riscv64gc",
            "riscv64imac",
            "x86_64",
            "",
        ] {
            assert!(IsaSpec::from_str(s).is_err(), "{s:?} should be rejected");
        }
    }

    #[test]
    fn rejects_bad_suffix() {
        assert!(IsaSpec::from_str("riscv32im_wjCus0_zve32x_zvl128b").is_err());
        assert!(IsaSpec::from_str("riscv64im_foo").is_err());
    }
}
