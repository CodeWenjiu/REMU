use std::sync::Arc;

use remu_harness::{SimulatorDut, SimulatorRef};

pub trait DebuggerRunner {
    fn run<D, R>(
        self,
        option: crate::DebuggerOption,
        interrupt: Arc<std::sync::atomic::AtomicBool>,
    ) where
        D: SimulatorDut,
        R: SimulatorRef<D::Policy>;
}
