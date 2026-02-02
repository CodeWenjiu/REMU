pub use remu_harness::{HarnessPolicy, SimulatorTrait};

pub trait DebuggerRunner {
    fn run<P: HarnessPolicy, R: SimulatorTrait<P>>(self, option: crate::DebuggerOption);
}
