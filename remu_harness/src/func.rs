use remu_simulator::{FuncCmd, TraceCmd};
use remu_types::TraceFlags;

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
    pub flags: TraceFlags,
}

impl Trace {
    fn new() -> Self {
        Self {
            flags: TraceFlags::new(),
        }
    }

    fn execute(&mut self, command: &TraceCmd) {
        match command {
            TraceCmd::Instruction { enable } => self.flags.set_instruction(*enable),
            TraceCmd::WaveForm { enable } => self.flags.set_waveform(*enable),
        }
    }
}
