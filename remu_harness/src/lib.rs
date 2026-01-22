remu_macro::mod_flat!(option, command);

use remu_simulator::Simulator;
use remu_types::TracerDyn;

pub struct Harness {
    simulator: Simulator,
}

impl Harness {
    pub fn new(opt: HarnessOption, tracer: TracerDyn) -> Self {
        Self {
            simulator: Simulator::new(opt.simulator, tracer),
        }
    }

    pub fn execute(&mut self, command: &Command) {
        match command {
            Command::State { subcmd } => self.simulator.get_state_mut().execute(subcmd),
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
