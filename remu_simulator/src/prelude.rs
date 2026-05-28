//! Public API surface of `remu_simulator`. Import via `use remu_simulator::prelude::*;`.

pub use crate::error::{
    DifftestMismatchList, SimulatorError, SimulatorInnerError, from_state_error,
};
pub use crate::func::{FuncCmd, TraceCmd};
pub use crate::option::SimulatorOption;
pub use crate::policy::{SimulatorPolicy, SimulatorPolicyOf};
pub use crate::simulator_trait::{SimulatorCore, SimulatorDut, SimulatorRef};
pub use crate::stat::{StatCmd, StatContext, StatEntry};
