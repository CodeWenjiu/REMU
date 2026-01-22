use crate::{bus::BusOption, reg::RegOption};

#[derive(clap::Args, Debug, Clone)]
pub struct StateOption {
    /// Bus Option
    #[command(flatten)]
    pub bus: BusOption,

    /// Register Option
    #[command(flatten)]
    pub reg: RegOption,
}
