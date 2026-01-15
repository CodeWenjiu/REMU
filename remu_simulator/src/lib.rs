use remu_state::{State, StateOption};

/// As a template
pub struct Simulator {
    state: State,
}

impl Simulator {
    pub fn new(opt: SimulatorOption) -> Self {
        Simulator {
            state: State::new(opt.state),
        }
    }

    pub fn get_state(&self) -> &State {
        &self.state
    }

    pub fn get_state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}

#[derive(clap::Args, Debug)]
pub struct SimulatorOption {
    /// State Option
    #[command(flatten)]
    pub state: StateOption,
}
