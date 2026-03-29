//! Classify [`IsaSpec`](remu_types::isa::IsaSpec) for the **remu** simulator backend (full matrix).
//!
//! Implements [`IsaKind`](remu_types::isa::IsaKind); nzea uses [`NzeaIsaKind`](remu_simulator_nzea::NzeaIsaKind).

use remu_types::isa::{ExtensionSpec, IsaKind, IsaSpec};
use target_lexicon::{Architecture, Riscv32Architecture};

/// Every ISA combination the remu CPU model can run today (see `remu_boot` dispatch).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemuIsaKind {
    Rv32I,
    Rv32Im,
    Rv32IWjCus0,
    Rv32ImWjCus0,
    Rv32IZve32xZvl128b,
    Rv32ImZve32xZvl128b,
}

impl IsaKind for RemuIsaKind {
    fn from_isa_spec_or_panic(spec: &IsaSpec) -> Self {
        use Architecture::Riscv32;
        use ExtensionSpec::*;
        use Riscv32Architecture::*;
        match (spec.base, spec.extensions) {
            (Riscv32(Riscv32i), None) => Self::Rv32I,
            (Riscv32(Riscv32im), None) => Self::Rv32Im,
            (Riscv32(Riscv32i), WjCus0) => Self::Rv32IWjCus0,
            (Riscv32(Riscv32im), WjCus0) => Self::Rv32ImWjCus0,
            (Riscv32(Riscv32i), Zve32xZvl128b) => Self::Rv32IZve32xZvl128b,
            (Riscv32(Riscv32im), Zve32xZvl128b) => Self::Rv32ImZve32xZvl128b,
            (arch, ext) => panic!(
                "unsupported ISA for remu simulator: base={:?}, extensions={:?}",
                arch, ext
            ),
        }
    }
}
