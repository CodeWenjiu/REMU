use crate::bus::{Bus, BusOption};

remu_macro::mod_pub!(bus);

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

    pub fn hello(&self) {
        tracing::info!("hello state");
    }
}

#[derive(clap::Args, Debug)]
pub struct StateOption {
    /// Bus Option
    #[command(flatten)]
    pub bus: BusOption,
}
