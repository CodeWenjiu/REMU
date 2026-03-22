use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "xtask",
    about = "Print shell snippets for remu_hal (eval in workspace root)",
    version,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Print(PrintCli),
}

#[derive(Debug, clap::Args)]
pub struct PrintCli {
    #[command(subcommand)]
    pub cmd: PrintCmd,
}

#[derive(Debug, Subcommand)]
pub enum PrintCmd {
    RunApp(RunAppArgs),
    BuildApp(BuildAppArgs),
    RunRemu(RunRemuArgs),
}

#[derive(Debug, clap::Args)]
pub struct BuildAppArgs {
    pub app: String,
    pub target: String,
}

#[derive(Debug, clap::Args)]
pub struct RunAppArgs {
    pub app: String,
    pub target: String,
}

#[derive(Debug, clap::Args)]
pub struct RunRemuArgs {
    pub elf_path: PathBuf,
}
