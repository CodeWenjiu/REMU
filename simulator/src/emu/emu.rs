use bitflags::bitflags;
use logger::Logger;
use option_parser::OptionParser;
use owo_colors::OwoColorize;
use remu_macro::{log_err, log_error};
use remu_utils::{ProcessError, ProcessResult, ISA};
use state::{reg::RegfileIo, States};

use crate::{SimulatorCallback, SimulatorItem};

use super::isa::riscv::{EmuPipeCell, RISCV};

bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct InstructionSetFlags: u8 {
        /// RV32I base integer instruction set
        const RV32I = 1 << 0;

        const RV32ILS = 1 << 1;
        /// RV32M integer multiplication and division extension
        const RV32M = 1 << 2;
        /// RV32E base integer instruction set (embedded)
        const RV32E = 1 << 3;
        /// Zicsr control and status register extension
        const ZICSR = 1 << 4;
        /// Privileged architecture extension
        const PRIV  = 1 << 5;
    }
}

impl From<ISA> for InstructionSetFlags {
    fn from(isa: ISA) -> Self {
        match isa {
            // RV32I always includes ZICSR and PRIV extensions
            ISA::RV32I => InstructionSetFlags::RV32I
                .union(InstructionSetFlags::ZICSR)
                .union(InstructionSetFlags::PRIV),
            
            // RV32IM includes RV32I plus the M extension
            ISA::RV32IM => InstructionSetFlags::RV32I
                .union(InstructionSetFlags::ZICSR)
                .union(InstructionSetFlags::PRIV)
                .union(InstructionSetFlags::RV32M),
            
            // RV32E always includes ZICSR and PRIV extensions
            ISA::RV32E => InstructionSetFlags::RV32E
                .union(InstructionSetFlags::ZICSR)
                .union(InstructionSetFlags::PRIV),
        }
    }
}

impl InstructionSetFlags {
    /// Check if the given instruction set is enabled
    pub fn enable(&self, isa: RISCV) -> bool {
        match isa {
            // RV32I instructions are enabled if either RV32I or RV32E is set
            RISCV::RV32I(_) => self.contains(InstructionSetFlags::RV32I),

            // RV32E instructions are enabled if RV32E is set
            RISCV::RV32E(_) => self.contains(InstructionSetFlags::RV32E),
            
            // Other extensions require their specific flag
            RISCV::RV32M(_) => self.contains(InstructionSetFlags::RV32M),
            RISCV::Priv(_) => self.contains(InstructionSetFlags::PRIV),
            RISCV::Zicsr(_) => self.contains(InstructionSetFlags::ZICSR),
        }
    }
}

pub struct EmuTimes {
    /// Number of cycles to execute
    pub cycles: u64,
    
    /// Number of instructions executed
    pub instructions: u64,
}

/// RISC-V Emulator implementation
pub struct Emu {
    /// Enabled instruction set extensions
    pub instruction_set: InstructionSetFlags,
    
    /// Emulator state (registers, memory, etc.)
    pub states: States,
    
    /// Callbacks for emulator events
    pub callback: SimulatorCallback,

    /// Emulator times
    pub times: EmuTimes,

    /// Emulator pipeline
    pub pipe: EmuPipeCell,
}

impl SimulatorItem for Emu {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        self.self_step_cycle()
    }

    fn times(&self) -> ProcessResult<()> {
        println!("{}: {}", "Cycles".purple(), self.times.cycles.blue());
        println!("{}: {}", "Instructions".purple(), self.times.instructions.blue());
        Ok(())
    }

    fn function_wave_trace(&self,_enable:bool) {
        log_error!("Wave tracing is not supported in Emu.");
    }
}

impl Emu {
    /// Create a new Emu instance
    pub fn new(option: &OptionParser, states: States, callback: SimulatorCallback) -> Self {
        let isa = option.cli.platform.isa;

        Self {
            instruction_set: InstructionSetFlags::from(isa),
            states,
            callback,
            times: EmuTimes {
                cycles: 0,
                instructions: 0,
            },
            pipe: EmuPipeCell::default(),
        }
    }

    /// Check if there's a conflict between instruction set extensions
    fn is_conflict(&self, set_flag: InstructionSetFlags) -> bool {
        // RV32E and RV32I are mutually exclusive
        if self.instruction_set.contains(InstructionSetFlags::RV32E) && 
           set_flag.contains(InstructionSetFlags::RV32I) {
            return true;
        }
        
        // RV32I is required for RV32M
        if !self.instruction_set.contains(InstructionSetFlags::RV32I) && 
           set_flag.contains(InstructionSetFlags::RV32M) {
            return true;
        }
        
        false
    }

    /// Set the instruction set extensions
    pub fn set_instruction_set(&mut self, set_flag: ISA) -> Result<(), ()> {
        let set_flag = InstructionSetFlags::from(set_flag);

        if self.is_conflict(set_flag) {
            return Err(());
        }

        self.instruction_set = self.instruction_set | set_flag;
        
        Ok(())
    }

    /// Execute a single cycle in the emulator
    pub fn self_step_cycle(&mut self) -> ProcessResult<()> {
        // 1. Fetch: Read the PC and fetch the instruction

        let pc = self.states.regfile.read_pc();
        let inst = log_err!(
            self.states.mmu.read(pc, state::mmu::Mask::Word), 
            ProcessError::Recoverable
        )?;

        // 2. Decode: Decode the instruction
        let decode = self.decode(pc, inst.1)?;
        
        // 3. Execute: Execute the instruction
        let next_pc = self.execute(decode)?;

        // 4. Notify completion and return
        (self.callback.instruction_complete)(pc, next_pc, inst.1)?;

        self.times.cycles += 1;
        self.times.instructions += 1;

        Ok(())
    }
}
