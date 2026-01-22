remu_macro::mod_flat!(option);

use remu_state::State;
use remu_types::TracerDyn;

/// As a template
pub struct Simulator {
    state: State,
    _tracer: TracerDyn,
}

impl Simulator {
    pub fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self {
        Simulator {
            state: State::new(opt.state, tracer.clone()),
            _tracer: tracer,
        }
    }

    pub fn get_state(&self) -> &State {
        &self.state
    }

    pub fn get_state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    pub fn step(&mut self, times: usize) {
        tracing::info!("Step {}", times);
    }
}
