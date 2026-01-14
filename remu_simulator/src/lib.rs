use remu_state::State;

/// As a template
pub struct Simulator {
    state: State,
}

impl Simulator {
    pub fn new() -> Self {
        Simulator {
            state: State::new(),
        }
    }

    pub fn get_state(&self) -> &State {
        &self.state
    }

    pub fn get_state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}
