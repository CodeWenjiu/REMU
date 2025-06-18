use remu_macro::log_err;
use remu_utils::{ProcessError, ProcessResult};
use state::reg::{riscv::{RvCsrEnum, Trap}, RegfileIo};

use crate::emu::{extract_bits, isa::riscv::{backend::{AlCtrl, LsCtrl, WbCtrl}, instruction::Priv}, sig_extend, Emu, InstructionSetFlags};

use super::{
    super::instruction::{ImmType, Zicsr, RISCV, RV32I, RV32IAL, RV32ILS, RV32M, }, InstType, IsCtrl, IsLogic, ToIsStage, SRCA, SRCB
};

#[derive(Default, Clone, Copy, Debug)]
pub struct ToIdStage {
    pub pc: u32,
    pub inst: u32,
}

impl Emu {
    /// Extract and sign-extend immediate value based on instruction type
    pub fn get_imm(inst: u32, imm_type: ImmType) -> u32 {
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

    /// Decode an instruction into its components
    pub fn instruction_decode(&self, msg: ToIdStage) -> ProcessResult<ToIsStage> {
        let pc = msg.pc;
        let instruction = msg.inst;

        // Extract register fields
        let rs1_addr = if self.instruction_set.contains(InstructionSetFlags::RV32E) {
            extract_bits(instruction, 15..18)
        } else {
            extract_bits(instruction, 15..19)
        } as u8;
        let rs2_addr = if self.instruction_set.contains(InstructionSetFlags::RV32E) {
            extract_bits(instruction, 20..23)
        } else {
            extract_bits(instruction, 20..24)
        } as u8;

        let decode_result = self.instruction_parse(instruction);

        let trap = match decode_result {
            None => Some(Trap::IllegalInstruction),

            Some((opcode, _)) => {
                match opcode {
                    RISCV::RV32I(RV32I::AL(inst)) | RISCV::RV32E(RV32I::AL(inst)) => {
                        match inst {
                            RV32IAL::Ecall => Some(Trap::EcallM),
                            RV32IAL::Ebreak => Some(Trap::Ebreak),
                            _ => None,
                        }
                    }

                    _ => None,
                }
            }
        };

        let (opcode, imm_type) = decode_result.unwrap_or((RISCV::RV32I(RV32I::AL(RV32IAL::Addi)), ImmType::I));

        let gpr_waddr = match opcode {
            RISCV::RV32I(RV32I::AL(inst)) | RISCV::RV32E(RV32I::AL(inst)) => {
                match inst {
                    RV32IAL::Beq | RV32IAL::Bne  | RV32IAL::Blt  | RV32IAL::Bge  | RV32IAL::Bltu | RV32IAL::Bgeu => 0,

                    RV32IAL::Fence => 0, // do nothing for now

                    _ => extract_bits(instruction, 7..11) as u8,
                }
            }

            _ => extract_bits(instruction, 7..11) as u8,
        };
            
        // Extract immediate value
        let imm = match opcode {
            RISCV::Priv(inst) => {
                match inst {
                    Priv::Mret => RvCsrEnum::MEPC.into(),
                }
            }

            _ => Self::get_imm(instruction, imm_type)
        };

        let regfile = &self.states.regfile;

        let rs1_val = log_err!(regfile.read_gpr(rs1_addr as u32), ProcessError::Recoverable)?;
        let rs2_val = log_err!(regfile.read_gpr(rs2_addr as u32), ProcessError::Recoverable)?;

        let logic = match opcode {
            RISCV::RV32I(RV32I::AL(inst)) | RISCV::RV32E(RV32I::AL(inst)) => {
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
            RISCV::RV32I(RV32I::AL(_)) | RISCV::RV32E(RV32I::AL(_)) => InstType::AL,
            RISCV::RV32I(RV32I::LS(_)) | RISCV::RV32E(RV32I::LS(_)) => InstType::LS,
            _ => InstType::AL,
        };

        let srca = match opcode {
            RISCV::RV32I(RV32I::AL(inst)) | RISCV::RV32E(RV32I::AL(inst)) => {
                match inst {
                    RV32IAL::Auipc | RV32IAL::Jal |
                    RV32IAL::Beq | RV32IAL::Bne  | RV32IAL::Blt  | RV32IAL::Bge  | RV32IAL::Bltu | RV32IAL::Bgeu => SRCA::PC,
                    _ => SRCA::RS1,
                }
            }

            RISCV::Zicsr(_) => SRCA::CSR,

            RISCV::Priv(_) => SRCA::CSR,

            _ => SRCA::RS1,
        };

        let srcb = match opcode {
            RISCV::RV32I(RV32I::AL(inst)) | RISCV::RV32E(RV32I::AL(inst)) => {
                match inst {
                    RV32IAL::Lui  | RV32IAL::Auipc | RV32IAL::Jal | RV32IAL::Jalr | RV32IAL::Addi |
                    RV32IAL::Xori | RV32IAL::Ori   | RV32IAL::Andi| 
                    RV32IAL::Slli | RV32IAL::Srli  | RV32IAL::Srai => SRCB::IMM,

                    RV32IAL::Beq  | RV32IAL::Bne  | RV32IAL::Blt  | RV32IAL::Bge  | RV32IAL::Bltu | RV32IAL::Bgeu => SRCB::LogicBranch,
                    
                    RV32IAL::Slti | RV32IAL::Sltiu | RV32IAL::Slt | RV32IAL::Sltu => SRCB::LogicSet,

                    _ => SRCB::RS2,
                }
            }

            RISCV::Zicsr(_) => SRCB::RS1,

            RISCV::Priv(_) => SRCB::RS1,

            _ => SRCB::RS2,
        };

        let is_ctrl = IsCtrl {
            inst_type,
            srca,
            srcb,
            logic,
        };

        let al_ctrl = match opcode {
            RISCV::RV32I(RV32I::AL(inst)) | RISCV::RV32E(RV32I::AL(inst)) => {
                match inst {
                    RV32IAL::Xori | RV32IAL::Xor => AlCtrl::Xor,
                    RV32IAL::Ori  | RV32IAL::Or  => AlCtrl::Or,
                    RV32IAL::Andi | RV32IAL::And => AlCtrl::And,
                    RV32IAL::Slli | RV32IAL::Sll => AlCtrl::Sll,
                    RV32IAL::Srli | RV32IAL::Srl => AlCtrl::Srl,
                    RV32IAL::Srai | RV32IAL::Sra => AlCtrl::Sra,

                    RV32IAL::Sub  => AlCtrl::Sub,

                    RV32IAL::Auipc | 
                    RV32IAL::Jal | RV32IAL::Jalr | 
                    RV32IAL::Beq | RV32IAL::Bne  | RV32IAL::Blt  | RV32IAL::Bge  | RV32IAL::Bltu | RV32IAL::Bgeu |
                    RV32IAL::Add | RV32IAL::Addi => AlCtrl::Add,

                    RV32IAL::Lui | RV32IAL::Slti | RV32IAL::Sltiu | RV32IAL::Slt | RV32IAL::Sltu => AlCtrl::B,

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

            RISCV::Zicsr(inst) => {
                match inst {
                    Zicsr::Csrrw => AlCtrl::B,
                    Zicsr::Csrrs => AlCtrl::Or,

                    _ => AlCtrl::DontCare,
                }
            }

            RISCV::Priv(_) => AlCtrl::Add,

            _ => AlCtrl::DontCare,
        };

        let ls_ctrl = match opcode {
            RISCV::RV32I(RV32I::LS(inst)) | RISCV::RV32E(RV32I::LS(inst)) => {
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
            RISCV::RV32I(RV32I::AL(inst)) | RISCV::RV32E(RV32I::AL(inst)) => {
                match inst {
                    RV32IAL::Jal | RV32IAL::Jalr | 
                    RV32IAL::Beq | RV32IAL::Bne  | RV32IAL::Blt  | RV32IAL::Bge  | RV32IAL::Bltu | RV32IAL::Bgeu => WbCtrl::Jump,
                    RV32IAL::Fence | RV32IAL::Ecall | RV32IAL::Ebreak => WbCtrl::DontCare,
                    _ => WbCtrl::WriteGpr,
                }
            }

            RISCV::RV32M(_) => WbCtrl::WriteGpr,

            RISCV::Zicsr(_) => WbCtrl::Csr,

            RISCV::Priv(_) => WbCtrl::Jump,

            _ => WbCtrl::DontCare,
        };

        Ok(ToIsStage { pc, rs1_addr, rs1_val, rs2_addr, rs2_val, gpr_waddr, imm, is_ctrl, al_ctrl, ls_ctrl, wb_ctrl, trap })
    }

}
