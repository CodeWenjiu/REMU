use remu_utils::{ProcessError, ProcessResult};
use state::reg::{riscv::Trap, RegfileIo};

use crate::emu::{extract_bits, isa::riscv::backend::{AlCtrl, LsCtrl, WbCtrl}, sig_extend, Emu, InstructionSetFlags};

use super::{
    super::{ImmType, Priv, Zicsr, RISCV, RV32I, RV32IAL, RV32ILS, RV32M, }, InstType, IsCtrl, IsLogic, ToIsStage, RV32_IAL_PATTERN_ITER, RV32_ILS_PATTERN_ITER, RV32_M_PATTERN_ITER, RV_PRIV_PATTERN_ITER, RV_ZICSR_PATTERN_ITER, SRCA, SRCB
};

#[derive(Default, Clone, Copy)]
pub struct ToIdStage {
    pub pc: u32,
    pub inst: u32,
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
    pub fn instruction_decode(&self, msg: ToIdStage) -> ProcessResult<ToIsStage> {
        let pc = msg.pc;
        let inst = msg.inst;
        if let Some((opcode, imm_type)) = self.isa_decode(inst) {
            // Extract register fields
            let rs1_addr = extract_bits(inst, 15..19) as u8;
            let rs2_addr = extract_bits(inst, 20..24) as u8;

            let gpr_waddr = match opcode {
                RISCV::RV32I(RV32I::AL(opcode)) => {
                    match opcode {
                        RV32IAL::Beq | RV32IAL::Bne  | RV32IAL::Blt  | RV32IAL::Bge  | RV32IAL::Bltu | RV32IAL::Bgeu => 0,

                        RV32IAL::Fence => 0, // do nothing for now

                        _ => extract_bits(inst, 7..11) as u8,
                    }
                }

                _ => extract_bits(inst, 7..11) as u8,
            };
            
            // Extract immediate value
            let imm = Self::get_imm(inst, imm_type);

            let regfile = &self.states.regfile;
            let rs1_val: u32 = regfile.read_gpr(rs1_addr.into()).map_err(|_| ProcessError::Recoverable)?;
            let rs2_val: u32 = regfile.read_gpr(rs2_addr.into()).map_err(|_| ProcessError::Recoverable)?;

            let logic = match opcode {
                RISCV::RV32I(RV32I::AL(inst)) => {
                    match inst {
                        RV32IAL::Beq => IsLogic::EQ,
                        RV32IAL::Bne => IsLogic::NE,
                        RV32IAL::Blt | RV32IAL::Slt => IsLogic::LT,
                        RV32IAL::Bge => IsLogic::GE,
                        RV32IAL::Bltu | RV32IAL::Sltu => IsLogic::LTU,
                        RV32IAL::Bgeu => IsLogic::GEU,
                        RV32IAL::Slti => IsLogic::SLTI,
                        RV32IAL::Sltiu => IsLogic::SLTIU,

                        _ => IsLogic::DontCare,
                    }
                }
                _ => IsLogic::DontCare,
            };

            let inst_type = match opcode {
                RISCV::RV32I(RV32I::AL(_)) => InstType::AL,
                RISCV::RV32I(RV32I::LS(_)) => InstType::LS,
                _ => InstType::AL,
            };

            let srca = match opcode {
                RISCV::RV32I(RV32I::AL(inst)) => {
                    match inst {
                        RV32IAL::Lui | RV32IAL::Slti | RV32IAL::Sltiu | RV32IAL::Slt | RV32IAL::Sltu => SRCA::ZERO,
                        RV32IAL::Auipc | RV32IAL::Jal |
                        RV32IAL::Beq | RV32IAL::Bne  | RV32IAL::Blt  | RV32IAL::Bge  | RV32IAL::Bltu | RV32IAL::Bgeu => SRCA::PC,
                        _ => SRCA::RS1,
                    }
                }
                _ => SRCA::RS1,
            };

            let srcb = match opcode {
                RISCV::RV32I(RV32I::AL(inst)) => {
                    match inst {
                        RV32IAL::Lui  | RV32IAL::Auipc | RV32IAL::Jal | RV32IAL::Jalr | RV32IAL::Addi |
                        RV32IAL::Xori | RV32IAL::Ori   | RV32IAL::Andi| 
                        RV32IAL::Slli | RV32IAL::Srli  | RV32IAL::Srai => SRCB::IMM,

                        RV32IAL::Beq  | RV32IAL::Bne  | RV32IAL::Blt  | RV32IAL::Bge  | RV32IAL::Bltu | RV32IAL::Bgeu => SRCB::LogicBranch,
                        
                        RV32IAL::Slti | RV32IAL::Sltiu | RV32IAL::Slt | RV32IAL::Sltu => SRCB::LogicSet,

                        _ => SRCB::RS2,
                    }
                }

                _ => SRCB::RS2,
            };

            let is_ctrl = IsCtrl {
                inst_type,
                srca,
                srcb,
                logic,
            };

            let al_ctrl = match opcode {
                RISCV::RV32I(RV32I::AL(inst)) => {
                    match inst {
                        RV32IAL::Xori | RV32IAL::Xor => AlCtrl::Xor,
                        RV32IAL::Ori  | RV32IAL::Or  => AlCtrl::Or,
                        RV32IAL::Andi | RV32IAL::And => AlCtrl::And,
                        RV32IAL::Slli | RV32IAL::Sll => AlCtrl::Sll,
                        RV32IAL::Srli | RV32IAL::Srl => AlCtrl::Srl,
                        RV32IAL::Srai | RV32IAL::Sra => AlCtrl::Sra,

                        RV32IAL::Sub  => AlCtrl::Sub,

                        RV32IAL::Lui  | RV32IAL::Auipc | 
                        RV32IAL::Jal | RV32IAL::Jalr | 
                        RV32IAL::Beq | RV32IAL::Bne  | RV32IAL::Blt  | RV32IAL::Bge  | RV32IAL::Bltu | RV32IAL::Bgeu |
                        RV32IAL::Slti | RV32IAL::Sltiu | RV32IAL::Slt | RV32IAL::Sltu |
                        RV32IAL::Add | RV32IAL::Addi => AlCtrl::Add,

                        _ => AlCtrl::DontCare,
                    }
                }

                RISCV::RV32M(inst) => {
                    match inst {
                        RV32M::Mul               => AlCtrl::Mul,
                        RV32M::Mulh              => AlCtrl::Mulh,
                        RV32M::Mulhsu            => AlCtrl::Mulhsu,
                        RV32M::Mulhu             => AlCtrl::Mulhu,

                        RV32M::Div               => AlCtrl::Div,
                        RV32M::Divu              => AlCtrl::Divu,

                        RV32M::Rem               => AlCtrl::Rem,
                        RV32M::Remu              => AlCtrl::Remu,
                    }
                }

                _ => AlCtrl::DontCare,
            };

            let ls_ctrl = match opcode {
                RISCV::RV32I(RV32I::LS(inst)) => {
                    match inst {
                        RV32ILS::Lb => LsCtrl::Lb,
                        RV32ILS::Lh => LsCtrl::Lh,
                        RV32ILS::Lw => LsCtrl::Lw,

                        RV32ILS::Lbu => LsCtrl::Lbu,
                        RV32ILS::Lhu => LsCtrl::Lhu,

                        RV32ILS::Sb => LsCtrl::Sb,
                        RV32ILS::Sh => LsCtrl::Sh,
                        RV32ILS::Sw => LsCtrl::Sw,
                    }
                }

                _ => LsCtrl::DontCare,
            };

            let wb_ctrl = match opcode {
                RISCV::RV32I(RV32I::AL(inst)) => {
                    match inst {
                        RV32IAL::Jal | RV32IAL::Jalr | 
                        RV32IAL::Beq | RV32IAL::Bne  | RV32IAL::Blt  | RV32IAL::Bge  | RV32IAL::Bltu | RV32IAL::Bgeu => WbCtrl::Jump,
                        RV32IAL::Fence | RV32IAL::Ecall | RV32IAL::Ebreak => WbCtrl::DontCare,
                        _ => WbCtrl::WriteGpr,
                    }
                }

                RISCV::RV32M(_) => WbCtrl::WriteGpr,

                _ => WbCtrl::DontCare,
            };

            let trap = match opcode {
                RISCV::RV32I(RV32I::AL(inst)) => {
                    match inst {
                        RV32IAL::Ecall => {
                            Some(Trap::EcallM)
                        }
            
                        RV32IAL::Ebreak => {
                            Some(Trap::Ebreak)
                        }

                        _ => None,
                    }
                }

                _ => None,
            };

            // Create instruction pattern
            Ok(ToIsStage { pc, rs1_addr, rs1_val, rs2_addr, rs2_val, gpr_waddr, imm, is_ctrl, al_ctrl, ls_ctrl, wb_ctrl, trap })
        } else {
            // Decoding failed
            (self.callback.decode_failed)(pc, inst);
            Err(ProcessError::Recoverable)
        }
    }

}
