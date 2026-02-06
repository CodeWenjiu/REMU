remu_macro::mod_flat!(option, policy, simulator_trait, error, func);

pub use error::{from_state_error, DifftestMismatchList, SimulatorError, SimulatorInnerError};
pub use policy::{SimulatorPolicy, SimulatorPolicyOf};
pub use simulator_trait::SimulatorTrait;
