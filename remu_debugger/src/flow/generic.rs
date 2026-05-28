use std::sync::Arc;

use remu_harness::PlatformConfig;

pub trait DebuggerRunner {
    fn run_with_config<C: PlatformConfig>(
        self,
        option: crate::DebuggerOption,
        interrupt: Arc<std::sync::atomic::AtomicBool>,
    );
}
