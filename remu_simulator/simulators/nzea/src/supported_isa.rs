//! Which [`IsaSpec`](remu_types::isa::IsaSpec) values the nzea Verilated backend accepts.

use remu_types::isa::{ExtensionSpec, IsaKind, IsaSpec};
use target_lexicon::{Architecture, Riscv32Architecture};

/// nzea models **riscv32i** / **riscv32im**, optionally with **`wjCus0`** suffix (same RTL as base).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NzeaIsaKind {
    Rv32I,
    Rv32Im,
    Rv32IWjCus0,
    Rv32ImWjCus0,
}

impl NzeaIsaKind {
    /// `None` if this ISA is not supported on nzea.
    pub fn try_from_isa_spec(spec: &IsaSpec) -> Option<Self> {
        match (spec.base, spec.extensions) {
            (Architecture::Riscv32(Riscv32Architecture::Riscv32i), ExtensionSpec::None) => {
                Some(Self::Rv32I)
            }
            (Architecture::Riscv32(Riscv32Architecture::Riscv32im), ExtensionSpec::None) => {
                Some(Self::Rv32Im)
            }
            (Architecture::Riscv32(Riscv32Architecture::Riscv32i), ExtensionSpec::WjCus0) => {
                Some(Self::Rv32IWjCus0)
            }
            (Architecture::Riscv32(Riscv32Architecture::Riscv32im), ExtensionSpec::WjCus0) => {
                Some(Self::Rv32ImWjCus0)
            }
            _ => None,
        }
    }
}

impl IsaKind for NzeaIsaKind {
    fn from_isa_spec_or_panic(spec: &IsaSpec) -> Self {
        Self::try_from_isa_spec(spec).unwrap_or_else(|| {
            panic!(
                "nzea supports riscv32i/riscv32im with optional _wjCus0 only; got base={:?}, extensions={:?}",
                spec.base, spec.extensions
            )
        })
    }
}
