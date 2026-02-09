use std::sync::Arc;

pub use remu_harness::{HarnessPolicy, SimulatorTrait};

pub trait DebuggerRunner {
    fn run<P: HarnessPolicy, R: SimulatorTrait<P, false>>(
        self,
        option: crate::DebuggerOption,
        interrupt: Arc<std::sync::atomic::AtomicBool>,
    );
}
