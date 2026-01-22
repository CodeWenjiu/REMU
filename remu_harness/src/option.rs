use remu_simulator::SimulatorOption;

#[derive(clap::Args, Debug, Clone)]
pub struct HarnessOption {
    /// Simulator Option
    #[command(flatten)]
    pub simulator: SimulatorOption,
}
