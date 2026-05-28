//! Type-level platform configuration — bundles Policy, Dut, and Ref into one type parameter.
//!
//! Each concrete `PlatformConfig` impl answers: which ISA, which observer mode,
//! and which DUT/Ref pair to wire into Harness and Debugger.
//!
//! Inspired by the runtime `SimulatorOption` / `StateOption` pattern:
//! the upper layer passes a config type down, and each layer unpacks what it needs.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use remu_types::TracerDyn;

use crate::option::SimulatorOption;
use crate::policy::SimulatorPolicy;
use crate::simulator_trait::{SimulatorDut, SimulatorRef};

pub trait PlatformConfig {
    type Policy: SimulatorPolicy;
    type Dut: SimulatorDut<Policy = Self::Policy>;
    type Ref: SimulatorRef<Self::Policy>;

    fn create_dut(
        opt: &SimulatorOption,
        tracer: TracerDyn,
        interrupt: Arc<AtomicBool>,
    ) -> Self::Dut;

    fn create_ref(
        opt: &SimulatorOption,
        tracer: TracerDyn,
        interrupt: Arc<AtomicBool>,
    ) -> Self::Ref;
}
