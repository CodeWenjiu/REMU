use crate::Simulator;

use bitflags::bitflags;
use remu_utils::ISA;
bitflags! {
    #[derive(Clone, Copy, Debug)]
    struct InstructionSetFlags: u8 {
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
    instruction_set: InstructionSetFlags,
}

impl Simulator for Emu {
}

impl Emu {
    pub fn new(isa: ISA) -> Self {
        Self {
            instruction_set: InstructionSetFlags::from(isa),
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
