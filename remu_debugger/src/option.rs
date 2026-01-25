use clap::builder::styling;
use remu_harness::HarnessOption;

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
pub struct RemuOptionParer {
    /// Harness Option
    #[command(flatten)]
    pub harness: HarnessOption,
}
