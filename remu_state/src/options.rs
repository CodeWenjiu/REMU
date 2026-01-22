use crate::bus::BusOption;

#[derive(clap::Args, Debug, Clone)]
pub struct StateOption {
    /// Bus Option
    #[command(flatten)]
    pub bus: BusOption,
}
