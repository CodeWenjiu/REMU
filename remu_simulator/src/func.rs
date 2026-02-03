use clap::ArgAction;

#[derive(Debug, clap::Subcommand)]
pub enum FuncCmd {
    /// Trace Command
    Trace {
        #[command(subcommand)]
        subcmd: TraceCmd,
    },

    /// Print All Function State
    Print,
}

#[derive(Debug, clap::Subcommand)]
pub enum TraceCmd {
    /// Instruction Trace
    Instruction {
        #[arg(value_parser = parse_switch, action = ArgAction::Set)]
        enable: bool,
    },
}

fn parse_switch(s: &str) -> Result<bool, String> {
    match s {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(format!("Invalid switch value: {}", s)),
    }
}
