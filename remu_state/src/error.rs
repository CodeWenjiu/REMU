use std::backtrace::Backtrace;
use thiserror::Error;

use crate::bus::BusError;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("bus error: {0}")]
    BusError(Box<BusError>),

    /// Execution stopped at a breakpoint (DUT debugger). PC where ebreak was hit.
    #[error("breakpoint hit at 0x{0:08x}")]
    BreakpointHit(u32),

    /// CSR index is not in `remu_types::isa::reg::csr::Csr` / not wired in `remu_state` yet.
    #[error(
        "unimplemented CSR at PC 0x{pc:08x} (csr_addr = 0x{csr_addr:03x}, decoded CSR immediate field = 0x{imm_raw:08x})"
    )]
    UnimplementedCsr {
        pc: u32,
        csr_addr: u16,
        imm_raw: u32,
    },
}

impl From<BusError> for StateError {
    #[inline(always)]
    fn from(e: BusError) -> Self {
        StateError::BusError(Box::new(e))
    }
}

impl StateError {
    #[inline(always)]
    pub fn backtrace(&self) -> Option<&Backtrace> {
        match self {
            StateError::BusError(b) => b.backtrace(),
            StateError::BreakpointHit(_) | StateError::UnimplementedCsr { .. } => None,
        }
    }

    #[inline(always)]
    pub fn exit_code(&self) -> Option<remu_types::ExitCode> {
        match self {
            StateError::BusError(b) => match b.as_ref() {
                BusError::ProgramExit(ec) => Some(*ec),
                _ => None,
            },
            StateError::BreakpointHit(_) | StateError::UnimplementedCsr { .. } => None,
        }
    }

    #[inline(always)]
    pub fn breakpoint_pc(&self) -> Option<u32> {
        match self {
            StateError::BreakpointHit(pc) => Some(*pc),
            StateError::BusError(_) | StateError::UnimplementedCsr { .. } => None,
        }
    }
}
