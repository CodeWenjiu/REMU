//! Public API surface of `remu_simulator`. Import via `use remu_simulator::prelude::*;`.

pub use crate::SimulatorOption;
pub use crate::SimulatorPolicy;
pub use crate::{
    BreakpointErrorKind, DifftestMismatchList, RefErrorKind, SimulatorError, SimulatorInnerError,
    from_state_error,
};
pub use crate::{FuncCmd, TraceCmd};
pub use crate::{SimulatorCore, SimulatorDut, SimulatorRef};
pub use crate::{StatCmd, StatContext, StatEntry};
