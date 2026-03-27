//! Which [`IsaSpec`](remu_types::isa::IsaSpec) values the nzea Verilated backend accepts.

use remu_types::isa::{ExtensionSpec, IsaKind, IsaSpec};
use target_lexicon::{Architecture, Riscv32Architecture};

/// nzea only models **riscv32i** and **riscv32im** with no extra extension suffix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NzeaIsaKind {
    Rv32I,
    Rv32Im,
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
            _ => None,
        }
    }
}

impl IsaKind for NzeaIsaKind {
    fn from_isa_spec_or_panic(spec: &IsaSpec) -> Self {
        Self::try_from_isa_spec(spec).unwrap_or_else(|| {
            panic!(
                "nzea only supports riscv32i and riscv32im without extensions; got base={:?}, extensions={:?}",
                spec.base, spec.extensions
            )
        })
    }
}
