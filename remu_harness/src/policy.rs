use remu_state::{StateFastProfile, StateMmioProfile};
use remu_types::isa::RvIsa;

use crate::SimulatorPolicy;

pub trait HarnessPolicy: SimulatorPolicy {}

impl<ISA> HarnessPolicy for StateFastProfile<ISA> where ISA: RvIsa {}

impl<ISA> HarnessPolicy for StateMmioProfile<ISA> where ISA: RvIsa {}
