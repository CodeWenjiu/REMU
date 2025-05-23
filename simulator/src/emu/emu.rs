use bitflags::bitflags;
use logger::Logger;
use option_parser::OptionParser;
use owo_colors::OwoColorize;
use remu_macro::log_error;
use remu_utils::{ProcessResult, ISA};
use state::{reg::RegfileIo, States};

use crate::{SimulatorCallback, SimulatorItem};

use super::isa::riscv::{frontend::ToIfStage, backend::{AlInst, ToAgStage, ToAlStage}, RISCV, RV32I};

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
}

impl SimulatorItem for Emu {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        self.self_step_cycle_singlecycle()
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

    pub fn self_step_cycle_singlecycle(&mut self) -> ProcessResult<()> {
        let pc = self.states.regfile.read_pc();

        let to_if = ToIfStage { pc };

        let to_id = self.instruction_fetch_rv32i(to_if)?;

        let inst = to_id.inst;
        
        let to_ex = self.instruction_decode(to_id)?;
        let to_wb = match to_ex.inst {
            RISCV::RV32I(RV32I::AL(inst)) => {
                let to_al = ToAlStage {
                    pc: to_ex.pc,
                    inst: AlInst::RV32I(inst),
                    msg: to_ex.msg,
                };

                self.arithmetic_logic_rv32(to_al)?
            }
            
            RISCV::RV32I(RV32I::LS(inst)) => {
                let to_ag = ToAgStage {
                    pc: to_ex.pc,
                    inst,
                    msg: to_ex.msg,
                };

                let to_ls = self.address_generation_rv32i(to_ag)?;
                self.load_store_rv32i(to_ls)?
            }

            RISCV::RV32M(inst) => {
                let to_al = ToAlStage {
                    pc: to_ex.pc,
                    inst: AlInst::RV32M(inst),
                    msg: to_ex.msg,
                };
                self.arithmetic_logic_rv32(to_al)?
            }

            _ => unreachable!("{:?}", to_ex.inst),
        };

        let next_pc = self.write_back_rv32i(to_wb)?;
        
        (self.callback.instruction_complete)(pc, next_pc, inst)?;

        self.times.cycles += 1;
        self.times.instructions += 1;

        Ok(())
    }
}
