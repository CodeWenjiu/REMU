//! Nzea simulator: minimal framework, to be extended with actual execution logic.

use remu_state::State;
use remu_types::TracerDyn;

use remu_simulator::{
    SimulatorCore, SimulatorOption, SimulatorPolicy, SimulatorPolicyOf,
};

/// Nzea simulator instance. Holds CPU/memory state; step logic to be implemented.
pub struct SimulatorNzea<P: SimulatorPolicy, const IS_DUT: bool> {
    state: State<P>,
    _tracer: TracerDyn,
}

impl<P: SimulatorPolicy, const IS_DUT: bool> SimulatorPolicyOf for SimulatorNzea<P, IS_DUT> {
    type Policy = P;
}

impl<P: SimulatorPolicy, const IS_DUT: bool> SimulatorCore<P> for SimulatorNzea<P, IS_DUT> {
    fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self {
        Self {
            state: State::new(opt.state.clone(), tracer.clone(), IS_DUT),
            _tracer: tracer,
        }
    }

    fn state(&self) -> &State<P> {
        &self.state
    }

    fn state_mut(&mut self) -> &mut State<P> {
        &mut self.state
    }

    fn step_once<const ITRACE: bool>(&mut self) -> Result<(), remu_simulator::SimulatorInnerError> {
        let _ = ITRACE;
        // TODO: fetch, decode, execute one instruction
        Ok(())
    }
}
