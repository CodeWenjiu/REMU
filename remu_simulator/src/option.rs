use clap::builder::styling;
use remu_state::StateOption;
use remu_types::IsaSpec;

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
pub struct SimulatorOption {
    /// State Option
    #[command(flatten)]
    pub state: StateOption,

    /// ISA Option
    #[arg(long, default_value = "riscv32i")]
    pub isa: IsaSpec,
}
