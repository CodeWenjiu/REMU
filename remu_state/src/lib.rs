use crate::bus::{Bus, BusOption};

remu_macro::mod_pub!(bus);
remu_macro::mod_flat!(commands);

/// State template
pub struct State {
    pub bus: Bus,
}

impl State {
    pub fn new(opt: StateOption) -> Self {
        Self {
            bus: Bus::new(opt.bus),
        }
    }

    pub fn execute(&mut self, subcmd: &StateCmds) {
        match subcmd {
            StateCmds::Hello => tracing::info!("hello state"),
        }
    }
}

#[derive(clap::Args, Debug)]
pub struct StateOption {
    /// Bus Option
    #[command(flatten)]
    pub bus: BusOption,
}
