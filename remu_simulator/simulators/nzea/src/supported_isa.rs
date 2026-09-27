//! Which [`IsaSpec`](remu_isa::isa::IsaSpec) values the nzea Verilated backend accepts.

use remu_isa::isa::{ExtensionSpec, IsaBase, IsaKind, IsaSpec};

/// nzea models **riscv32i** / **riscv32im** / **riscv64i** / **riscv64im**.
/// The RV64 variants exist in the RTL once nzea lands 64-bit support; remu
/// side is wired for them (see `RV64-ABI.md` for the pending ABI decision).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NzeaIsaKind {
    Rv32I,
    Rv32Im,
    Rv64I,
    Rv64Im,
}

impl NzeaIsaKind {
    /// `None` if this ISA is not supported on nzea.
    pub fn try_from_isa_spec(spec: &IsaSpec) -> Option<Self> {
        match (spec.base, spec.extensions) {
            (IsaBase::Rv32I, ExtensionSpec::None) => Some(Self::Rv32I),
            (IsaBase::Rv32Im, ExtensionSpec::None) => Some(Self::Rv32Im),
            (IsaBase::Rv64I, ExtensionSpec::None) => Some(Self::Rv64I),
            (IsaBase::Rv64Im, ExtensionSpec::None) => Some(Self::Rv64Im),
            _ => None,
        }
    }
}

impl IsaKind for NzeaIsaKind {
    fn from_isa_spec_or_panic(spec: &IsaSpec) -> Self {
        Self::try_from_isa_spec(spec).unwrap_or_else(|| {
            panic!(
                "nzea supports riscv32i/riscv32im/riscv64i/riscv64im only; got base={:?}, extensions={:?}",
                spec.base, spec.extensions
            )
        })
    }
}
