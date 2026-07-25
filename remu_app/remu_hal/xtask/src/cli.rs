use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::platform::Platform;

#[derive(Debug, Parser)]
#[command(
    name = "xtask",
    about = "Print shell snippets for remu_hal (eval in workspace root)",
    version,
    propagate_version = true
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    Print(PrintCli),
}

#[derive(Debug, clap::Args)]
pub(crate) struct PrintCli {
    #[command(subcommand)]
    pub cmd: PrintCmd,
}

#[derive(Debug, Subcommand)]
pub(crate) enum PrintCmd {
    RunApp(RunAppArgs),
    BuildApp(BuildAppArgs),
    RunRemu(RunRemuArgs),
}

#[derive(Debug, clap::Args)]
pub(crate) struct BuildAppArgs {
    pub app: String,
    pub target: String,
    /// Target platform (remu, qemu, spike, host). Affects linker flags and features.
    #[arg(long = "platform", default_value = "remu")]
    pub platform: Platform,
}

#[derive(Debug, clap::Args)]
pub(crate) struct RunAppArgs {
    pub app: String,
    pub target: String,
    /// Target platform.
    #[arg(long = "platform", default_value = "remu")]
    pub platform: Platform,
    /// Extra args forwarded to remu_cli.
    #[arg(last = true)]
    pub remu_cli_args: Vec<String>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct RunRemuArgs {
    pub elf_path: PathBuf,
    /// Extra args forwarded to remu_cli (already tokenized by caller).
    #[arg(last = true)]
    pub remu_cli_args: Vec<String>,
}
