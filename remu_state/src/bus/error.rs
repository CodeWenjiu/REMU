use thiserror::Error;

use crate::bus::{AccessKind, MemFault};

/// In-memory fault type returned by RAM-backed `Memory` operations.
///
/// This is intentionally ISA-agnostic. The simulator/CPU layer should map it to an ISA trap.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BusFault {
    #[error("unmapped range: 0x{addr:016x} : 0x")]
    Unmapped { addr: usize },

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
    },

    #[error("Memory Fault {0}")]
    MemError(#[from] MemFault),
}
