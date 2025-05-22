use remu_utils::{ProcessError, ProcessResult};

use crate::emu::{extract_bits, sig_extend, Emu, InstructionSetFlags};

use super::{
    ImmType, InstPattern, Priv, Zicsr, RISCV, RV32I, RV32IAL, RV32ILS, RV32M, RV32_IAL_PATTERN_ITER, RV32_ILS_PATTERN_ITER, RV32_M_PATTERN_ITER, RV_PRIV_PATTERN_ITER, RV_ZICSR_PATTERN_ITER
};

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

    fn rv32_i_decode(inst: u32) -> Option<(RV32I, ImmType)> {
        if let Some((opcode, imm_type)) = Self::rv32_i_al_decode(inst) {
            return Some((RV32I::AL(opcode), imm_type));
        } else if let Some((opcode, imm_type)) = Self::rv32_i_ls_decode(inst) {
            return Some((RV32I::LS(opcode), imm_type));
        }

        None
    }

    /// Decode an instruction as RV32M
    fn rv32_m_decode(inst: u32) -> Option<(RV32M, ImmType)> {
        for (opcode, imm_type, (mask, value)) in RV32_M_PATTERN_ITER {
            if (inst & mask) == *value {
                return Some((*opcode, *imm_type));
            }
        }
        None
    }

    /// Decode an instruction as Zicsr
    fn rv_zicsr_decode(inst: u32) -> Option<(Zicsr, ImmType)> {
        for (opcode, imm_type, (mask, value)) in RV_ZICSR_PATTERN_ITER {
            if (inst & mask) == *value {
                return Some((*opcode, *imm_type));
            }
        }
        None
    }

    /// Decode an instruction as privileged
    fn rv_priv_decode(inst: u32) -> Option<(Priv, ImmType)> {
        for (opcode, imm_type, (mask, value)) in RV_PRIV_PATTERN_ITER {
            if (inst & mask) == *value {
                return Some((*opcode, *imm_type));
            }
        }
        None
    }

    /// Decode an instruction based on the enabled instruction set extensions
    fn isa_decode(&self, inst: u32) -> Option<(RISCV, ImmType)> {
        let isa = self.instruction_set;
        
        // Try to decode as RV32I first (most common)
        if isa.contains(InstructionSetFlags::RV32I) {
            if let Some((opcode, imm_type)) = Self::rv32_i_decode(inst) {
                return Some((RISCV::RV32I(opcode), imm_type));
            }
        }

        if isa.contains(InstructionSetFlags::RV32E) {
            if let Some((opcode, imm_type)) = Self::rv32_i_decode(inst) {
                return Some((RISCV::RV32E(opcode), imm_type));
            }
        }

        // Try to decode as RV32M
        if isa.contains(InstructionSetFlags::RV32M) {
            if let Some((opcode, imm_type)) = Self::rv32_m_decode(inst) {
                return Some((RISCV::RV32M(opcode), imm_type));
            }
        }

        // Try to decode as Zicsr
        if isa.contains(InstructionSetFlags::ZICSR) {
            if let Some((opcode, imm_type)) = Self::rv_zicsr_decode(inst) {
                return Some((RISCV::Zicsr(opcode), imm_type));
            }
        }

        // Try to decode as privileged
        if isa.contains(InstructionSetFlags::PRIV) {
            if let Some((opcode, imm_type)) = Self::rv_priv_decode(inst) {
                return Some((RISCV::Priv(opcode), imm_type));
            }
        }

        // Failed to decode
        None
    }

    /// Extract and sign-extend immediate value based on instruction type
    fn get_imm(inst: u32, imm_type: ImmType) -> u32 {
        match imm_type {
            // I-type: Load, ALU immediate, JALR
            ImmType::I => {
                let range = 20..31;
                let imm = extract_bits(inst, range.clone());
                sig_extend(imm, range.end as u8 - range.start as u8 + 1)
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

    /// Decode an instruction into its components
    pub fn decode(&mut self, pc: u32, inst: u32) -> ProcessResult<InstPattern> {
        if let Some((opcode, imm_type)) = self.isa_decode(inst) {
            // Extract register fields
            let rs1 = extract_bits(inst, 15..19) as u8;
            let rs2 = extract_bits(inst, 20..24) as u8;
            let rd = extract_bits(inst, 7..11) as u8;
            
            // Extract immediate value
            let imm = Self::get_imm(inst, imm_type);

            // Create instruction pattern
            Ok(InstPattern::new(opcode, rs1, rs2, rd, imm))
        } else {
            // Decoding failed
            (self.callback.decode_failed)(pc, inst);
            Err(ProcessError::Recoverable)
        }
    }
}
