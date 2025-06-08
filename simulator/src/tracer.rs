use cfg_if::cfg_if;
use owo_colors::OwoColorize;
use remu_utils::{ItraceConfigtionalWrapper, ProcessError, ProcessResult};

cfg_if! {
    if #[cfg(feature = "ITRACE")] {
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TraceFunction {
    #[cfg(feature = "ITRACE")]
    InstructionTrace,
}

#[derive(Clone)]
pub struct Tracer {
    #[cfg(feature = "ITRACE")]
    pub instruction_trace_enable: bool,
    
    pub conditional: ItraceConfigtionalWrapper,

    pub breakpoints: Vec<u32>,
}

impl Tracer {
    pub fn new(
        #[cfg(feature = "ITRACE")]
        instruction_trace_enable: bool,
        conditional: ItraceConfigtionalWrapper,
    ) -> Self {
        Self {
            #[cfg(feature = "ITRACE")]
            instruction_trace_enable,
            conditional,
            breakpoints: Vec::new(),
        }
    }

    #[cfg(feature = "ITRACE")]
    fn instruction_trace(&self, pc:u32, inst: u32) {
        if self.instruction_trace_enable {
            println!(
                "0x{:08x}: {}",
                pc.blue(),
                self.conditional.disassembler.borrow().try_analize(inst, pc).purple()
            );
        }
    }

    pub fn add_breakpoint(&mut self, addr: u32) {
        if let Some(pos) = self.breakpoints.iter().position(|&x| x == addr) {
            println!("Breakpoint already exists at {}:{:#010x}", pos.purple(), addr.blue());
        } else {
            let index = self.breakpoints.len();
            self.breakpoints.push(addr);
            println!("Breakpoint {} added at {:#010x}", index.purple(), addr.blue());
        }
    }

    pub fn remove_breakpoint_by_addr(&mut self, addr: u32) {
        if let Some(pos) = self.breakpoints.iter().position(|&x| x == addr) {
            self.breakpoints.remove(pos);
            println!("Breakpoint at {}:{:#010x} removed", pos.purple(), addr.blue());
        }
    }

    pub fn remove_breakpoint_by_index(&mut self, index: usize) {
        if index < self.breakpoints.len() {
            self.breakpoints.remove(index);
        }
    }

    pub fn show_breakpoints(&self) {
        println!("{}", "Breakpoints".purple());
        self
            .breakpoints
            .iter()
            .enumerate()
            .for_each(|(i, &addr)| {
                println!("{}: {:#010x}", i.purple(), addr.blue());
            });
    }

    pub fn check_breakpoint(&self, pc: u32) -> ProcessResult<()> {
        if let Some((i, _)) = self
            .breakpoints
            .iter()
            .enumerate()
            .find(|(_, addr)| **addr == pc)
        {
            println!("Hit Breakpoint {}: {:#010x}", i.purple(), pc.blue());
            Err(ProcessError::Recoverable)
        } else {
            Ok(())
        }
    }

    pub fn trace(&self, pc: u32, inst: u32) -> ProcessResult<()> {
        #[cfg(feature = "ITRACE")]
        self.instruction_trace(pc, inst);

        Ok(())
    }

    // ignore: although there is no function need to enable if not ITRACE for now, but it's better to keep it
    pub fn trace_function(
        &mut self,
        function: TraceFunction,
        _enable: bool,
    )  {
        match function {
            #[cfg(feature = "ITRACE")]
            TraceFunction::InstructionTrace => {
                self.instruction_trace_enable = _enable;
            }
        }
    }
}
