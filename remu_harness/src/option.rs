use remu_simulator::SimulatorOption;
use remu_types::IsaSpec;

#[derive(clap::Args, Debug, Clone)]
pub struct HarnessOption {
    /// Simulator Option
    #[command(flatten)]
    pub simulator: SimulatorOption,

    /// ISA Option
    #[arg(long, default_value = "riscv32i")]
    pub isa: IsaSpec,
}
