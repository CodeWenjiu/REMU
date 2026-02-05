use std::backtrace::Backtrace;
use thiserror::Error;

use crate::bus::{AccessKind, MemFault};

/// In-memory fault type returned by RAM-backed `Memory` operations.
///
/// This is intentionally ISA-agnostic. The simulator/CPU layer should map it to an ISA trap.
/// Each variant carries a backtrace via thiserror's #[backtrace] (requires nightly + error_generic_member_access).
#[derive(Debug, Error)]
pub enum BusError {
    #[error("unmapped range: 0x{addr:016x} : 0x")]
    Unmapped {
        addr: usize,
        #[backtrace]
        backtrace: Backtrace,
    },

    #[error(
        "out of bounds {kind:?} at 0x{addr:016x} (size={size}) for region '{region}' \
         [0x{base:016x}..0x{end:016x})"
    )]
    OutOfBounds {
        kind: AccessKind,
        addr: usize,
        size: usize,
        region: String,
        base: usize,
        end: usize,
        #[backtrace]
        backtrace: Backtrace,
    },

    #[error("Memory Fault {0}")]
    MemError(#[source] MemFault, #[backtrace] Backtrace),

    #[error("Unsupported Access Width")]
    UnsupportedAccessWidth(usize, #[backtrace] Backtrace),

    #[error("IO Error")]
    IoError(#[backtrace] Backtrace),

    #[error("program exit with code {0}")]
    ProgramExit(u32),
}

impl BusError {
    #[inline(always)]
    pub fn unmapped(addr: usize) -> Self {
        BusError::Unmapped {
            addr,
            backtrace: Backtrace::capture(),
        }
    }

    #[inline(always)]
    pub fn backtrace(&self) -> Option<&Backtrace> {
        match self {
            BusError::Unmapped { backtrace, .. } => Some(backtrace),
            BusError::OutOfBounds { backtrace, .. } => Some(backtrace),
            BusError::MemError(_, backtrace) => Some(backtrace),
            BusError::UnsupportedAccessWidth(_, backtrace) => Some(backtrace),
            BusError::IoError(backtrace) => Some(backtrace),
            BusError::ProgramExit(_) => None,
        }
    }
}

impl From<MemFault> for BusError {
    fn from(m: MemFault) -> Self {
        BusError::MemError(m, Backtrace::capture())
    }
}
