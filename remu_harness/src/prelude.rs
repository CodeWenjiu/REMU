//! Public API surface of `remu_harness`. Import via `use remu_harness::prelude::*;`.
//!
//! Facade crate: re-exports preludes from simulator / state / types,
//! plus concrete simulator types and harness's own API.

pub use remu_simulator::PlatformConfig;
pub use remu_simulator::SimulatorCore;
pub use remu_simulator::SimulatorOption;
pub use remu_simulator::prelude::*;
pub use remu_simulator_nzea::SimulatorNzea;
pub use remu_simulator_remu::SimulatorRemu;
pub use remu_simulator_spike::SimulatorSpike;
pub use remu_state::prelude::*;
pub use remu_types::prelude::*;

pub use crate::HarnessOption;
pub use crate::HarnessPolicy;
pub use crate::error::HarnessError;
pub use crate::isa_dispatch::RemuIsaKind;
pub use crate::run_state::{RunOutcome, RunState};
