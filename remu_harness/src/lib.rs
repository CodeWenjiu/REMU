remu_macro::mod_flat!(commands);

use remu_simulator::Simulator;

pub struct Harness {
    simulator: Simulator,
}

impl Harness {
    pub fn new(opt: HarnessOption) -> Self {
        Self {
            simulator: Simulator::new(opt.simulator),
        }
    }

    pub fn execute(&mut self, subcmd: &HarnessCommands) {
        match subcmd {
            HarnessCommands::State { subcmd } => match subcmd {
                StateCmds::Hello => {
                    self.simulator.get_state().hello();
                }
            },
            HarnessCommands::Times { subcmd } => match subcmd {
                TimeCmds::Count { subcmd } => match subcmd {
                    TimeCountCmds::Test => {
                        tracing::info!("Time Count Test")
                    }
                },
            },
            HarnessCommands::Continue => {
                tracing::info!("Continuing execution...");
            }
        }
    }
}
