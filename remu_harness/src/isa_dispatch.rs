//! Classify [`IsaSpec`](remu_isa::isa::IsaSpec) for the **remu** simulator backend (full matrix).
//!
//! Implements [`IsaKind`](remu_isa::isa::IsaKind); nzea uses [`NzeaIsaKind`](remu_simulator_nzea::NzeaIsaKind).

use remu_isa::isa::{ExtensionSpec, IsaBase, IsaKind, IsaSpec};

/// Every ISA combination the remu CPU model can run today (see `remu_boot` dispatch).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemuIsaKind {
    Rv32I,
    Rv32Im,
    Rv32IWjCus0,
    Rv32ImWjCus0,
    Rv32IZve32xZvl128b,
    Rv32ImZve32xZvl128b,
    Rv64I,
    Rv64Im,
}

impl IsaKind for RemuIsaKind {
    fn from_isa_spec_or_panic(spec: &IsaSpec) -> Self {
        use IsaBase::*;
        match (spec.base, spec.extensions) {
            (Rv32I, ExtensionSpec::None) => Self::Rv32I,
            (Rv32Im, ExtensionSpec::None) => Self::Rv32Im,
            (Rv32I, ExtensionSpec::WjCus0) => Self::Rv32IWjCus0,
            (Rv32Im, ExtensionSpec::WjCus0) => Self::Rv32ImWjCus0,
            (Rv32I, ExtensionSpec::Zve32xZvl128b) => Self::Rv32IZve32xZvl128b,
            (Rv32Im, ExtensionSpec::Zve32xZvl128b) => Self::Rv32ImZve32xZvl128b,
            (Rv64I, ExtensionSpec::None) => Self::Rv64I,
            (Rv64Im, ExtensionSpec::None) => Self::Rv64Im,
            (base, ext) => {
                panic!("unsupported ISA for remu simulator: base={base:?}, extensions={ext:?}")
            }
        }
    }
}
