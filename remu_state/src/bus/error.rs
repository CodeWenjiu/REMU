use std::backtrace::Backtrace;
use thiserror::Error;

use crate::bus::{AccessKind, MemFault};

fn capture_backtrace() -> String {
    format!("{}", Backtrace::capture())
}

/// In-memory fault type returned by RAM-backed `Memory` operations.
///
/// This is intentionally ISA-agnostic. The simulator/CPU layer should map it to an ISA trap.
/// Each variant carries a backtrace (as string) captured at construction so the full call chain
/// can be printed when the error is reported (no cost on the happy path).
#[derive(Debug, Error, Clone)]
pub enum BusError {
    #[error("unmapped range: 0x{addr:016x} : 0x")]
    Unmapped { addr: usize, backtrace: String },

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
        backtrace: String,
    },

    #[error("Memory Fault {0}")]
    MemError(MemFault, String),

    #[error("Unsupported Access Width")]
    UnsupportedAccessWidth(usize, String),

    #[error("IO Error")]
    IoError(String),
}

impl BusError {
    /// Returns the backtrace (string) captured when this fault was created (for error reporting).
    #[inline(always)]
    pub fn backtrace(&self) -> &str {
        match self {
            BusError::Unmapped { backtrace, .. } => backtrace,
            BusError::OutOfBounds { backtrace, .. } => backtrace,
            BusError::MemError(_, backtrace) => backtrace,
            BusError::UnsupportedAccessWidth(_, backtrace) => backtrace,
            BusError::IoError(backtrace) => backtrace,
        }
    }
}

impl From<MemFault> for BusError {
    fn from(m: MemFault) -> Self {
        BusError::MemError(m, capture_backtrace())
    }
}

impl BusError {
    #[inline(always)]
    pub fn unmapped(addr: usize) -> Self {
        BusError::Unmapped {
            addr,
            backtrace: capture_backtrace(),
        }
    }
}
