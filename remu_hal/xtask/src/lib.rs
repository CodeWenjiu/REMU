use std::process::ExitCode;

mod isa_shorthand;

remu_macro::mod_pub![cli, paths, target, disasm, util];

pub mod commands {
    pub mod print;
}

pub fn run() -> ExitCode {
    use clap::Parser;
    match cli::Cli::parse().command {
        cli::Command::Print(p) => commands::print::run(p.cmd),
    }
}
