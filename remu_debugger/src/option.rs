use clap::builder::styling;
use remu_harness::HarnessOption;
use remu_types::{DifftestRef, isa::IsaSpec, Platform};

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
    /// Harness Option
    #[command(flatten)]
    pub sim: HarnessOption,

    /// ISA Option
    #[arg(long, default_value = "riscv32i")]
    pub isa: IsaSpec,

    /// Platform (DUT simulator): remu, spike, nzea
    #[arg(long, default_value = "remu")]
    pub platform: Platform,

    /// Difftest Option
    #[arg(long, value_name = "REF")]
    pub difftest: Option<DifftestRef>,

    /// Batch Mode
    #[arg(long)]
    pub batch: bool,

    /// Startup sequence: run this command expression after the debugger is created (tokens joined with spaces; e.g. --startup '{' state reg pc write 0x1000 '}')
    #[arg(long = "startup", value_name = "TOKEN", num_args = 1..)]
    pub startup: Vec<String>,
}
