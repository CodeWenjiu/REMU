use std::sync::Arc;

pub use remu_harness::{HarnessPolicy, SimulatorDut, SimulatorRef};

pub trait DebuggerRunner {
    fn run<D, R>(
        self,
        option: crate::DebuggerOption,
        interrupt: Arc<std::sync::atomic::AtomicBool>,
    ) where
        D: SimulatorDut,
        R: SimulatorRef<D::Policy>;
}
