remu_macro::mod_flat!(option, command);

use remu_simulator::{Simulator, new_simulator};
use remu_types::TracerDyn;

pub struct Harness {
    simulator: Box<dyn Simulator>,
}

impl Harness {
    pub fn new(opt: HarnessOption, tracer: TracerDyn) -> Self {
        Self {
            simulator: new_simulator(opt.simulator, opt.isa, tracer),
        }
    }

    pub fn execute(&mut self, command: &Command) {
        match command {
            Command::State { subcmd } => self.simulator.get_state_mut().execute(subcmd),
            Command::Func { subcmd } => self.simulator.func(subcmd),
            Command::Step { times: steps } => self.simulator.step(*steps),
            Command::Times { subcmd } => match subcmd {
                TimeCmd::Count { subcmd } => match subcmd {
                    TimeCountCmd::Test => {
                        tracing::info!("Time Count Test")
                    }
                },
            },
            Command::Continue => {
                self.simulator.step(0);
            }
        }
    }
}
