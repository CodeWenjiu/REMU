use std::marker::PhantomData;

use remu_simulator::SimulatorPolicy;
use remu_types::isa::RvIsa;

pub trait DebuggerPolicy {
    type SimPolicy: SimulatorPolicy;
}

pub struct DebuggerProfile<ISA, SimPolicy>
where
    ISA: RvIsa,
    SimPolicy: SimulatorPolicy,
{
    _marker_isa: PhantomData<(ISA, SimPolicy)>,
}

impl<ISA, SimPolicy> DebuggerPolicy for DebuggerProfile<ISA, SimPolicy>
where
    ISA: RvIsa,
    SimPolicy: SimulatorPolicy,
{
    type SimPolicy = SimPolicy;
}

pub trait DebuggerRunner {
    fn run<P: DebuggerPolicy>(self, option: crate::DebuggerOption);
}
