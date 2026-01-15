use clap::builder::styling;
use remu_simulator::SimulatorOption;

#[derive(clap::Parser, Debug)]
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
pub struct OptionParser {
    /// State Option
    #[command(flatten)]
    pub simulator: SimulatorOption,
}
