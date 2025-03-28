use crate::{SimulatorCallback, SimulatorItem};

use bitflags::bitflags;
use logger::Logger;
use option_parser::OptionParser;
use remu_macro::log_err;
use remu_utils::{ProcessError, ProcessResult, ISA};
use state::{reg::RegfileIo, States};

use super::isa::riscv::RISCV;
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

impl InstructionSetFlags {
    pub fn enable(&self, isa:RISCV) -> bool {
        match isa {
            RISCV::RV32I(_) => self.contains(InstructionSetFlags::RV32I) || self.contains(InstructionSetFlags::RV32E),
            RISCV::RV32M(_) => self.contains(InstructionSetFlags::RV32M),
            RISCV::Priv(_) => self.contains(InstructionSetFlags::PRIV),
            RISCV::Zicsr(_) => self.contains(InstructionSetFlags::ZICSR),
        }
    }
}

pub struct Emu {
    pub instruction_set: InstructionSetFlags,

    pub states: States,

    pub callback: SimulatorCallback,
}

impl SimulatorItem for Emu {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        self.self_step_cycle()
    }
}

impl Emu {
    pub fn new(option: &OptionParser, states: States, callback: SimulatorCallback) -> Self {
        let isa = option.cli.platform.isa;

        Self {
            instruction_set: InstructionSetFlags::from(isa),
            states,
            callback,
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
    
    pub fn self_step_cycle(&mut self) -> ProcessResult<()> {
        let pc = self.states.regfile.read_pc();

        let inst = log_err!(self.states.mmu.read(pc, state::mmu::Mask::Word), ProcessError::Recoverable)?;

        let decode = self.decode(pc, inst)?;
        
        self.execute(decode)?;

        (self.callback.instruction_compelete)(pc, inst)?;

        Ok(())
    }
}
