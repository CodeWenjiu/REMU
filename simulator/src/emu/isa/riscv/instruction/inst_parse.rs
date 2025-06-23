
use state::reg::riscv::Trap;

use crate::emu::{isa::riscv::instruction::{RV32_IAL_PATTERN_ITER, RV32_ILS_PATTERN_ITER, RV32_M_PATTERN_ITER, RV_PRIV_PATTERN_ITER, RV_ZICSR_PATTERN_ITER}, Emu, InstructionSetFlags};

use super::{ImmType, Priv, Zicsr, RISCV, RV32I, RV32IAL, RV32ILS, RV32M};

pub enum DecodeResult {
    Result((RISCV, ImmType)),
    Trap(Trap),
}

impl Emu {
    /// Decode an instruction as RV32I
    fn rv32_i_al_decode(inst: u32) -> Option<(RV32IAL, ImmType)> {
        for (opcode, imm_type, (mask, value)) in RV32_IAL_PATTERN_ITER {
            if (inst & mask) == *value {
                return Some((*opcode, *imm_type));
            }
        }
        None
    }

    fn rv32_i_ls_decode(inst: u32) -> Option<(RV32ILS, ImmType)> {
        for (opcode, imm_type, (mask, value)) in RV32_ILS_PATTERN_ITER {
            if (inst & mask) == *value {
                return Some((*opcode, *imm_type));
            }
        }
        None
    }

    fn rv32_i_parse(inst: u32) -> Option<(RV32I, ImmType)> {
        if let Some((opcode, imm_type)) = Self::rv32_i_al_decode(inst) {
            return Some((RV32I::AL(opcode), imm_type));
        } else if let Some((opcode, imm_type)) = Self::rv32_i_ls_decode(inst) {
            return Some((RV32I::LS(opcode), imm_type));
        }

        None
    }

    /// Decode an instruction as RV32M
    fn rv32_m_parse(inst: u32) -> Option<(RV32M, ImmType)> {
        for (opcode, imm_type, (mask, value)) in RV32_M_PATTERN_ITER {
            if (inst & mask) == *value {
                return Some((*opcode, *imm_type));
            }
        }
        None
    }

    /// Decode an instruction as Zicsr
    fn rv_zicsr_parse(inst: u32) -> Option<(Zicsr, ImmType)> {
        for (opcode, imm_type, (mask, value)) in RV_ZICSR_PATTERN_ITER {
            if (inst & mask) == *value {
                return Some((*opcode, *imm_type));
            }
        }
        None
    }

    /// Decode an instruction as privileged
    fn rv_priv_parse(inst: u32) -> Option<(Priv, ImmType)> {
        for (opcode, imm_type, (mask, value)) in RV_PRIV_PATTERN_ITER {
            if (inst & mask) == *value {
                return Some((*opcode, *imm_type));
            }
        }
        None
    }

    /// Decode an instruction based on the enabled instruction set extensions
    pub fn instruction_parse(&self, inst: u32) -> DecodeResult {
        let isa = self.instruction_set;
        
        // Try to decode as RV32I first (most common)
        if isa.contains(InstructionSetFlags::RV32I) {
            if let Some((opcode, imm_type)) = Self::rv32_i_parse(inst) {
                if opcode == RV32I::AL(RV32IAL::Ebreak) {
                    return DecodeResult::Trap(Trap::Ebreak);
                }
                return DecodeResult::Result((RISCV::RV32I(opcode), imm_type));
            }
        }

        if isa.contains(InstructionSetFlags::RV32E) {
            if let Some((opcode, imm_type)) = Self::rv32_i_parse(inst) {
                return DecodeResult::Result((RISCV::RV32E(opcode), imm_type));
            }
        }

        // Try to decode as RV32M
        if isa.contains(InstructionSetFlags::RV32M) {
            if let Some((opcode, imm_type)) = Self::rv32_m_parse(inst) {
                return DecodeResult::Result((RISCV::RV32M(opcode), imm_type));
            }
        }

        // Try to decode as Zicsr
        if isa.contains(InstructionSetFlags::ZICSR) {
            if let Some((opcode, imm_type)) = Self::rv_zicsr_parse(inst) {
                return DecodeResult::Result((RISCV::Zicsr(opcode), imm_type));
            }
        }

        // Try to decode as privileged
        if isa.contains(InstructionSetFlags::PRIV) {
            if let Some((opcode, imm_type)) = Self::rv_priv_parse(inst) {
                return DecodeResult::Result((RISCV::Priv(opcode), imm_type));
            }
        }

        DecodeResult::Trap(Trap::IllegalInstruction)
    }
}