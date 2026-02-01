use remu_state::{StateFastProfile, StateMmioProfile, StatePolicy};
use remu_types::isa::RvIsa;

pub trait SimulatorPolicy: StatePolicy {}

impl<ISA> SimulatorPolicy for StateFastProfile<ISA> where ISA: RvIsa {}

impl<ISA> SimulatorPolicy for StateMmioProfile<ISA> where ISA: RvIsa {}
