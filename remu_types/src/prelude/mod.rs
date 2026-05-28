//! Public API surface of `remu_types`. Import via `use remu_types::prelude::*;`.
//!
//! Rule: a symbol goes here iff it is imported by 2+ downstream crates,
//! or is the main entry-point type of this crate.

pub use crate::difftest::DifftestMismatchItem;
pub use crate::exit_code::ExitCode;
pub use crate::platform::Platform;
pub use crate::trace_flags::{TraceFlags, TraceKind};
pub use crate::wordlen::Xlen;
pub use crate::{AllUsize, DifftestRef, RegGroup, TracerDyn};
