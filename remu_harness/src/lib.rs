remu_macro::mod_flat!(commands);

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

    pub fn execute(&mut self, command: &Commands) {
        match command {
            Commands::State { subcmd } => self.simulator.get_state_mut().execute(subcmd),
            Commands::Times { subcmd } => match subcmd {
                TimeCmds::Count { subcmd } => match subcmd {
                    TimeCountCmds::Test => {
                        tracing::info!("Time Count Test")
                    }
                },
            },
            Commands::Continue => {
                self.simulator.step(0);
            }
        }
    }
}
