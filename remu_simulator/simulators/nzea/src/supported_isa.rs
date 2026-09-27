//! Which [`IsaSpec`](remu_isa::isa::IsaSpec) values the nzea Verilated backend accepts.

use remu_isa::isa::{ExtensionSpec, IsaBase, IsaKind, IsaSpec};

/// nzea models **riscv32i** / **riscv32im**.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NzeaIsaKind {
    Rv32I,
    Rv32Im,
}

impl NzeaIsaKind {
    /// `None` if this ISA is not supported on nzea.
    pub fn try_from_isa_spec(spec: &IsaSpec) -> Option<Self> {
        match (spec.base, spec.extensions) {
            (IsaBase::Rv32I, ExtensionSpec::None) => Some(Self::Rv32I),
            (IsaBase::Rv32Im, ExtensionSpec::None) => Some(Self::Rv32Im),
            _ => None,
        }
    }
}

impl IsaKind for NzeaIsaKind {
    fn from_isa_spec_or_panic(spec: &IsaSpec) -> Self {
        Self::try_from_isa_spec(spec).unwrap_or_else(|| {
            panic!(
                "nzea supports riscv32i/riscv32im only; got base={:?}, extensions={:?}",
                spec.base, spec.extensions
            )
        })
    }
}
