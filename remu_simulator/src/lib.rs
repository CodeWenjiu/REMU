remu_macro::mod_flat!(option, policy, simulator_trait, error, func);

pub use error::{DifftestMismatchList, SimulatorError};
pub use policy::{SimulatorPolicy, SimulatorPolicyOf};
pub use simulator_trait::SimulatorTrait;
