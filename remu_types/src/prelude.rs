//! Public API surface of `remu_types`. Import via `use remu_types::prelude::*;`.
//!
//! Rule: a symbol goes here iff it is imported by 2+ downstream crates,
//! or is the main entry-point type of this crate.

pub use crate::ExitCode;
pub use crate::Platform;
pub use crate::{AllUsize, TracerDyn};
pub use crate::{DifftestGroup, DifftestMismatchItem, DifftestRef, DifftestRegGroup};
pub use crate::{TraceFlags, TraceKind};
pub use remu_isa::Xlen;
