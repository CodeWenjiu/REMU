use std::process::ExitCode;

remu_macro::mod_prv!(
    app_caps,
    isa_shorthand,
    cli,
    paths,
    target,
    disasm,
    util,
    platform
);
remu_macro::mod_pub!(crate, commands);

use cli::{Cli, Command};

pub use platform::{Platform, PlatformConfig};

pub fn run() -> ExitCode {
    use clap::Parser;
    match Cli::parse().command {
        Command::Print(p) => commands::run(p.cmd),
    }
}
