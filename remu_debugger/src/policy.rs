use remu_state::{StateFastProfile, StateMmioProfile};
use remu_harness::{SimulatorPolicy, SimulatorTrait};
use remu_types::isa::RvIsa;

pub trait DebuggerPolicy: SimulatorPolicy {}

impl<ISA> DebuggerPolicy for StateFastProfile<ISA> where ISA: RvIsa {}

impl<ISA> DebuggerPolicy for StateMmioProfile<ISA> where ISA: RvIsa {}

pub trait DebuggerRunner {
    fn run<P: DebuggerPolicy, R: SimulatorTrait<P>>(self, option: crate::DebuggerOption);
}
