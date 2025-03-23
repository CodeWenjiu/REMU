use std::{cell::RefCell, rc::Rc};

use crate::{FunctionTarget, Simulator};

use bitflags::bitflags;
use logger::Logger;
use option_parser::{DebugConfiguration, OptionParser};
use remu_utils::{Disassembler, ProcessError, ProcessResult, ISA};
use state::States;
bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct InstructionSetFlags: u8 {
        const RV32I = 1 << 0;
        const RV32M = 1 << 1;
        const RV32E = 1 << 2;
        const ZICSR = 1 << 3;
        const PRIV  = 1 << 4;
    }
}

impl From<ISA> for InstructionSetFlags {
    fn from(isa: ISA) -> Self {
        match isa {
            ISA::RV32I => InstructionSetFlags::RV32I.union(InstructionSetFlags::ZICSR).union(InstructionSetFlags::PRIV),
            ISA::RV32IM => InstructionSetFlags::RV32I.union(InstructionSetFlags::ZICSR).union(InstructionSetFlags::PRIV).union(InstructionSetFlags::RV32M),
            ISA::RV32E => InstructionSetFlags::RV32E.union(InstructionSetFlags::ZICSR).union(InstructionSetFlags::PRIV),
        }
    }
}

pub struct Emu {
    pub instruction_set: InstructionSetFlags,

    pub instruction_trace_enable: bool,
    pub disaseembler: Rc<RefCell<Disassembler>>,
    states: Rc<RefCell<States>>,
}

impl Simulator for Emu {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        let pc = self.states.borrow().regfile.read_pc();

        let inst = self.states.borrow_mut().mmu.read(pc, state::mmu::Mask::Word).map_err(|e| {
            Logger::show(&e.to_string(), Logger::ERROR);
            ProcessError::Recoverable
        })?;

        let decode = self.decode(inst, pc)?;

        println!("{:?}", decode);

        if self.instruction_trace_enable {
            let disassembler = self.disaseembler.borrow();
            Logger::show(&format!("{}", disassembler.try_analize(inst, pc)).to_string(), Logger::INFO);
        }

        Ok(())
    }

    fn cmd_function_mut(&mut self, target:crate::FunctionTarget, enable:bool) -> ProcessResult<()> {
        match target {
            FunctionTarget::InstructionTrace => {
                self.instruction_trace(enable);
            }
        }

        Ok(())
    }
}

impl Emu {
    pub fn new(option: &OptionParser, states: Rc<RefCell<States>>, disaseembler: Rc<RefCell<Disassembler>>) -> Self {
        let isa = option.cli.platform.isa;

        let mut instruction_trace_enable = false;

        for debug_config in &option.cfg.debug_config {
            match debug_config {
                DebugConfiguration::Itrace { enable } => {
                    instruction_trace_enable = *enable;
                }

                _ => {
                }
            }
        }

        Self {
            instruction_set: InstructionSetFlags::from(isa),

            instruction_trace_enable,
            disaseembler,
            states,
        }
    }

    fn is_conflict(&self, set_flag: InstructionSetFlags) -> bool {
        if self.instruction_set.contains(InstructionSetFlags::RV32E) && set_flag.contains(InstructionSetFlags::RV32I) {
            return true;
        }
        if self.instruction_set.contains(InstructionSetFlags::RV32I) && set_flag.contains(InstructionSetFlags::RV32M) {
            return true;
        }
        false
    }

    pub fn set_instruction_set(&mut self, set_flag: ISA) -> Result<(), ()> {
        let set_flag = InstructionSetFlags::from(set_flag);

        if self.is_conflict(set_flag) {
            return Err(());
        }

        self.instruction_set = self.instruction_set | set_flag;
        
        Ok(())
    }
}
