use clap::{CommandFactory, Subcommand, builder::styling};

#[derive(clap::Parser, Debug)]
#[command(
    author,
    version,
    about,
    disable_help_flag = true,
    disable_version_flag = true,
    styles = styling::Styles::styled()
    .header(styling::AnsiColor::Green.on_default().bold())
    .usage(styling::AnsiColor::Green.on_default().bold())
    .literal(styling::AnsiColor::Blue.on_default().bold())
    .placeholder(styling::AnsiColor::Cyan.on_default())
)]
pub(crate) struct CommandParser {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    /// Print version information
    Version,

    /// continue the emulator
    Continue,

    /// Times printf
    Times {
        #[arg(default_value("1"))]
        count: u64,
    },
}

fn get_command_list() -> Vec<String> {
    let command = CommandParser::command();
    command
        .get_subcommands()
        .map(|sub| sub.get_name().to_string())
        .collect()
}

pub fn get_command_with_help() -> Vec<String> {
    let mut commands = get_command_list();
    commands.push("help".to_string());
    commands
}
