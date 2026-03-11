remu_macro::mod_flat!(option, policy, simulator_trait, error, func, stat);

pub use error::{from_state_error, DifftestMismatchList, SimulatorError, SimulatorInnerError};
pub use func::{FuncCmd, TraceCmd};
pub use policy::{SimulatorPolicy, SimulatorPolicyOf};
pub use simulator_trait::{SimulatorCore, SimulatorDut, SimulatorRef};
pub use stat::{StatCmd, StatContext, StatEntry};
