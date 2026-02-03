use remu_simulator::{FuncCmd, TraceCmd};

#[derive(Debug)]
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
            FuncCmd::Print => println!("Function State: {:?}", self),
        }
    }
}

#[derive(Debug)]
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
