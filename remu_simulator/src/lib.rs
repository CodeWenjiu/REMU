remu_macro::mod_pub!(riscv);
remu_macro::mod_flat!(option, policy, simulator_trait, func);

pub use policy::SimulatorPolicy;
pub use simulator_trait::{SimulatorDut, SimulatorRemu, SimulatorTrait};
