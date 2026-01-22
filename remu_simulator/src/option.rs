use remu_state::StateOption;

#[derive(clap::Args, Debug, Clone)]
pub struct SimulatorOption {
    /// State Option
    #[command(flatten)]
    pub state: StateOption,
}
