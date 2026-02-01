use clap::builder::styling;
use remu_simulator::SimulatorOption;
use remu_types::{DifftestRef, isa::IsaSpec};

#[derive(clap::Parser, Debug, Clone)]
#[command(
    author,
    version,
    about,
    styles = styling::Styles::styled()
    .header(styling::AnsiColor::Green.on_default().bold())
    .usage(styling::AnsiColor::Green.on_default().bold())
    .literal(styling::AnsiColor::Blue.on_default().bold())
    .placeholder(styling::AnsiColor::Cyan.on_default())
)]
pub struct DebuggerOption {
    /// Simulator Option
    #[command(flatten)]
    pub sim: SimulatorOption,

    /// ISA Option
    #[arg(long, default_value = "riscv32i")]
    pub isa: IsaSpec,

    /// Difftest Option
    #[arg(long, value_name = "REF")]
    pub difftest: Option<DifftestRef>,
}
