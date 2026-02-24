use remu_state::{StateFastProfile, StateMmioProfile, StatePolicy};
use remu_types::isa::RvIsa;

pub trait SimulatorPolicy: StatePolicy {}

impl<ISA> SimulatorPolicy for StateFastProfile<ISA> where ISA: RvIsa {}

impl<ISA> SimulatorPolicy for StateMmioProfile<ISA> where ISA: RvIsa {}

/// Extracts the `SimulatorPolicy` used by a concrete simulator type, for Harness etc. to constrain D/R without explicit P.
pub trait SimulatorPolicyOf {
    type Policy: SimulatorPolicy;
}
