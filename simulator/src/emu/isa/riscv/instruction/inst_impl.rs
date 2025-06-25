use state::reg::riscv::Trap;

use crate::emu::isa::riscv::instruction::DecodeResult;
use crate::emu::{extract_bits, sig_extend, InstructionSetFlags};

use crate::emu::{isa::riscv::instruction::{RV32_IAL_PATTERN_ITER, RV32_ILS_PATTERN_ITER, RV32_M_PATTERN_ITER, RV_PRIV_PATTERN_ITER, RV_ZICSR_PATTERN_ITER}};

use super::{ImmType, Priv, Zicsr, RISCV, RV32I, RV32IAL, RV32ILS, RV32M};

impl InstructionSetFlags {
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
        // Try to decode as RV32I first (most common)
        if self.contains(InstructionSetFlags::RV32I) {
            if let Some((opcode, imm_type)) = Self::rv32_i_parse(inst) {
                if opcode == RV32I::AL(RV32IAL::Ebreak) {
                    return DecodeResult::Trap(Trap::Ebreak);
                }
                return DecodeResult::Result((RISCV::RV32I(opcode), imm_type));
            }
        }

        if self.contains(InstructionSetFlags::RV32E) {
            if let Some((opcode, imm_type)) = Self::rv32_i_parse(inst) {
                return DecodeResult::Result((RISCV::RV32E(opcode), imm_type));
            }
        }

        // Try to decode as RV32M
        if self.contains(InstructionSetFlags::RV32M) {
            if let Some((opcode, imm_type)) = Self::rv32_m_parse(inst) {
                return DecodeResult::Result((RISCV::RV32M(opcode), imm_type));
            }
        }

        // Try to decode as Zicsr
        if self.contains(InstructionSetFlags::ZICSR) {
            if let Some((opcode, imm_type)) = Self::rv_zicsr_parse(inst) {
                return DecodeResult::Result((RISCV::Zicsr(opcode), imm_type));
            }
        }

        // Try to decode as privileged
        if self.contains(InstructionSetFlags::PRIV) {
            if let Some((opcode, imm_type)) = Self::rv_priv_parse(inst) {
                return DecodeResult::Result((RISCV::Priv(opcode), imm_type));
            }
        }

        DecodeResult::Trap(Trap::IllegalInstruction)
    }
}

pub trait ImmGet {
    fn get_imm(inst: u32, imm_type: ImmType) -> u32 {
        match imm_type {
            // I-type: Load, ALU immediate, JALR
            ImmType::I => {
                let range = 20..31;
                let imm = extract_bits(inst, range.clone());
                sig_extend(imm, range.end - range.start + 1)
            },
            
            // S-type: Store
            ImmType::S => {
                let imm = (extract_bits(inst, 25..31) << 5) | extract_bits(inst, 7..11);
                sig_extend(imm, 12)
            },
            
            // B-type: Branch
            ImmType::B => {
                let imm = (extract_bits(inst, 31..31) << 12) | 
                          (extract_bits(inst, 25..30) << 5) | 
                          (extract_bits(inst, 8..11) << 1) | 
                          (extract_bits(inst, 7..7) << 11);
                sig_extend(imm, 13)
            },
            
            // U-type: LUI, AUIPC
            ImmType::U => {
                extract_bits(inst, 12..31) << 12
            },
            
            // J-type: JAL
            ImmType::J => {
                let imm = (extract_bits(inst, 31..31) << 20) | 
                          (extract_bits(inst, 12..19) << 12) | 
                          (extract_bits(inst, 20..20) << 11) | 
                          (extract_bits(inst, 21..30) << 1);
                sig_extend(imm, 21)
            },
            
            // R-type: Register-register operations (no immediate)
            ImmType::R => 0,
            
            // N-type: No operands
            ImmType::N => 0,
        }
    }
}
