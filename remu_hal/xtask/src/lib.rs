use std::process::ExitCode;

remu_macro::mod_flat!(isa_shorthand, cli, paths, target, disasm, util);
remu_macro::mod_pub!(commands);

pub fn run() -> ExitCode {
    use clap::Parser;
    match Cli::parse().command {
        Command::Print(p) => commands::run(p.cmd),
    }
}
