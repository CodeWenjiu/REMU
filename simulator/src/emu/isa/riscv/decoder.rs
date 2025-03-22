use logger::Logger;
use remu_utils::{ProcessError, ProcessResult};

use crate::emu::{extract_bits, sig_extend, Emu, InstructionSetFlags};

use super::{ImmType, InstPattern, Priv, Zicsr, RISCV, RV32I, RV32M, RV32_I_PATTERN_ITER, RV32_M_PATTERN_ITER, RV_PRIV_PATTERN_ITER, RV_ZICSR_PATTERN_ITER};

impl Emu {
    fn rv32_i_decode(inst: u32) -> Option<(RV32I, ImmType)> {
        for (opcode, imm_type, (mask, value)) in RV32_I_PATTERN_ITER {
            if *opcode == RV32I::Auipc {
                println!("{:#b} {:#b}", inst & mask, *value);
            }
            if (inst & mask) == *value {
                return Some((opcode.clone(), imm_type.clone()));
            }
        }

        None
    }

    fn rv32_m_decode(inst: u32) -> Option<(RV32M, ImmType)> {
        for (opcode, imm_type, (mask, value)) in RV32_M_PATTERN_ITER {
            if (inst & mask) == *value {
                return Some((opcode.clone(), imm_type.clone()));
            }
        }

        None
    }

    fn rv_zicsr_decode(inst: u32) -> Option<(Zicsr, ImmType)> {
        for (opcode, imm_type, (mask, value)) in RV_ZICSR_PATTERN_ITER {
            if (inst & mask) == *value {
                return Some((opcode.clone(), imm_type.clone()));
            }
        }

        None
    }

    fn rv_priv_decode(inst: u32) -> Option<(Priv, ImmType)> {
        for (opcode, imm_type, (mask, value)) in RV_PRIV_PATTERN_ITER {
            if (inst & mask) == *value {
                return Some((opcode.clone(), imm_type.clone()));
            }
        }

        None
    }

    fn isa_decode(&self, inst: u32) -> Option<(RISCV, ImmType)> {
        let isa = self.instruction_set;
        
        if isa.contains(InstructionSetFlags::RV32I) {
            if let Some((opcode, imm_type)) = Self::rv32_i_decode(inst) {
                return Some((RISCV::RV32I(opcode), imm_type));
            }
        }

        if isa.contains(InstructionSetFlags::RV32M) {
            if let Some((opcode, imm_type)) = Self::rv32_m_decode(inst) {
                return Some((RISCV::RV32M(opcode), imm_type));
            }
        }

        if isa.contains(InstructionSetFlags::ZICSR) {
            if let Some((opcode, imm_type)) = Self::rv_zicsr_decode(inst) {
                return Some((RISCV::Zicsr(opcode), imm_type));
            }
        }

        if isa.contains(InstructionSetFlags::PRIV) {
            if let Some((opcode, imm_type)) = Self::rv_priv_decode(inst) {
                return Some((RISCV::Priv(opcode), imm_type));
            }
        }

        None
    }

    fn get_imm(inst: u32, imm_type: ImmType) -> u32 {
        match imm_type {
            ImmType::I => {
                let range = 20..31;
                let imm = extract_bits(inst, range.clone());
                sig_extend(imm, range.end as u8 - range.start as u8 + 1)
            },
            ImmType::S => {
                let imm = (extract_bits(inst, 25..31) << 5) | extract_bits(inst, 7..11);
                sig_extend(imm, 12)
            },
            ImmType::B => {
                let imm = (extract_bits(inst, 31..31) << 12) | (extract_bits(inst, 25..30) << 5) | (extract_bits(inst, 8..11) << 1) | (extract_bits(inst, 7..7) << 11);
                sig_extend(imm, 13)
            },
            ImmType::U => {
                extract_bits(inst, 12..31) << 12
            },
            ImmType::J => {
                let imm = (extract_bits(inst, 31..31) << 20) | (extract_bits(inst, 12..19) << 12) | (extract_bits(inst, 20..20) << 11) | (extract_bits(inst, 21..30) << 1);
                sig_extend(imm, 21)
            },
            ImmType::R => {
                0
            },
            ImmType::N => {
                0
            },
        }
    }

    pub fn decode(&self, inst: u32) -> ProcessResult<InstPattern> {
        if let Some((opcode, imm_type)) = self.isa_decode(inst) {
            let rs1 = extract_bits(inst, 15..19) as u8;
            let rs2 = extract_bits(inst, 20..24) as u8;
            let rd = extract_bits(inst, 7..11) as u8;
            let imm = Self::get_imm(inst, imm_type);

            Ok(InstPattern::new(opcode, rs1, rs2, rd, imm))
        } else {
            Logger::show(format!("Decode failed :{:#034b}", inst).as_str(), Logger::ERROR);
            Err(ProcessError::Recoverable)
        }
    }
}