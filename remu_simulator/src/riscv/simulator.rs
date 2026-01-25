use remu_state::State;
use remu_types::TracerDyn;

use crate::{Simulator, SimulatorOption};

/// As a template
pub(crate) struct SimulatorRiscv {
    state: State,
    _tracer: TracerDyn,
}

impl SimulatorRiscv {
    pub fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self {
        SimulatorRiscv {
            state: State::new(opt.state, tracer.clone()),
            _tracer: tracer,
        }
    }
}

impl Simulator for SimulatorRiscv {
    fn get_state(&self) -> &State {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    fn step(&mut self, times: usize) {
        tracing::info!("Step {}", times);
    }
}
