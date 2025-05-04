use std::{cell::RefCell, rc::Rc};

use owo_colors::OwoColorize;
use remu_utils::{Disassembler, ProcessError, ProcessResult};

#[derive(Debug, PartialEq, Clone)]
pub enum TraceFunction {
    InstructionTrace,
}

#[derive(Clone)]
pub struct Tracer {
    pub instruction_trace_enable: bool,
    pub disassembler: Rc<RefCell<Disassembler>>,

    pub breakpoints: Vec<u32>,
}

impl Tracer {
    pub fn new(
        instruction_trace_enable: bool,
        disassembler: Rc<RefCell<Disassembler>>
    ) -> Self {
        Self {
            instruction_trace_enable,
            disassembler,
            breakpoints: Vec::new(),
        }
    }

    fn instruction_trace(&self, pc:u32, inst: u32) {
        if self.instruction_trace_enable {
            println!(
                "0x{:08x}: {}",
                pc.blue(),
                self.disassembler.borrow().try_analize(inst, pc).purple()
            );
        }
    }

    pub fn add_breakpoint(&mut self, addr: u32) {
        if !self.breakpoints.contains(&addr) {
            self.breakpoints.push(addr);
        }
    }

    pub fn remove_breakpoint_by_addr(&mut self, addr: u32) {
        if let Some(pos) = self.breakpoints.iter().position(|&x| x == addr) {
            self.breakpoints.remove(pos);
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

    fn check_breakpoint(&self, pc: u32) -> ProcessResult<()> {
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
        self.instruction_trace(pc, inst);

        self.check_breakpoint(pc)?;

        Ok(())
    }

    pub fn trace_function(
        &mut self,
        function: TraceFunction,
        enable: bool,
    )  {
        match function {
            TraceFunction::InstructionTrace => {
                self.instruction_trace_enable = enable;
            }
        }
    }
}
