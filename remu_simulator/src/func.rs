use clap::ArgAction;

#[derive(Debug, clap::Subcommand)]
pub enum FuncCmd {
    /// Trace Command
    Trace {
        #[command(subcommand)]
        subcmd: TraceCmd,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum TraceCmd {
    /// Instruction Trace
    Instruction {
        #[arg(value_parser = parse_switch, action = ArgAction::Set)]
        enable: bool,
    },
}

pub(crate) struct Func {
    pub trace: Trace,
}

impl Func {
    pub fn new() -> Self {
        Self {
            trace: Trace::new(),
        }
    }

    pub fn execute(&mut self, command: &FuncCmd) {
        match command {
            FuncCmd::Trace { subcmd } => self.trace.execute(subcmd),
        }
    }
}

pub(crate) struct Trace {
    pub instruction: bool,
}

impl Trace {
    fn new() -> Self {
        Self { instruction: false }
    }

    fn execute(&mut self, command: &TraceCmd) {
        match command {
            TraceCmd::Instruction { enable } => self.instruction = *enable,
        }
    }
}

fn parse_switch(s: &str) -> Result<bool, String> {
    match s {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(format!("Invalid switch value: {}", s)),
    }
}
