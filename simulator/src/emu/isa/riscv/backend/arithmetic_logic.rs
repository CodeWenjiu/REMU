use remu_utils::ProcessResult;
use state::reg::{riscv::RvCsrEnum, RegfileIo};

use crate::emu::Emu;

use super::{ToWbStage, };
use super::super::{InstMsg, Trap, RV32IAL, RV32M};

pub enum AlInst {
    RV32I(RV32IAL),
    RV32M(RV32M),
}

impl Default for AlInst {
    fn default() -> Self {
        AlInst::RV32I(RV32IAL::default())
    }
}

#[derive(Default)]
pub struct ToAlStage {
    pub pc: u32,
    pub inst: AlInst,
    pub msg: InstMsg, 
}

impl Emu {
    pub fn arithmetic_logic_rv32i(&self, pc: u32, inst: RV32IAL, msg: InstMsg) -> ProcessResult<ToWbStage> {
        let mut next_pc = pc.wrapping_add(4);

        let imm = msg.imm;
        let rs1_val = msg.rs1;
        let rs2_val = msg.rs2;

        let mut rd_addr = msg.rd_addr;
        let mut rd_val = 0;

        let csr_wmsg = (false, 0, 0);

        let mut trap = None;

        match inst {
            RV32IAL::Lui => {
                rd_val = imm;
            }

            RV32IAL::Auipc => {
                rd_val = pc.wrapping_add(imm);
            }

            RV32IAL::Jal => {
                rd_val = next_pc;
                next_pc = pc.wrapping_add(imm);
            }

            RV32IAL::Jalr => {
                rd_val = next_pc;
                next_pc = rs1_val.wrapping_add(imm);
            }

            RV32IAL::Beq => {
                rd_addr = 0;
                if rs1_val == rs2_val {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32IAL::Bne => {
                rd_addr = 0;
                if rs1_val != rs2_val {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32IAL::Blt => {
                rd_addr = 0;
                if (rs1_val as i32) < (rs2_val as i32) {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32IAL::Bge => {
                rd_addr = 0;
                if (rs1_val as i32) >= (rs2_val as i32) {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32IAL::Bltu => {
                rd_addr = 0;
                if rs1_val < rs2_val {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32IAL::Bgeu => {
                rd_addr = 0;
                if rs1_val >= rs2_val {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32IAL::Addi => {
                rd_val = rs1_val.wrapping_add(imm);
            }

            RV32IAL::Slti => {
                rd_val = if (rs1_val as i32) < (imm as i32) { 1 } else { 0 };
            }

            RV32IAL::Sltiu => {
                rd_val = if rs1_val < imm { 1 } else { 0 };
            }

            RV32IAL::Xori => {
                rd_val = rs1_val ^ imm;
            }

            RV32IAL::Ori => {
                rd_val = rs1_val | imm;
            }

            RV32IAL::Andi => {
                rd_val = rs1_val & imm;
            }

            RV32IAL::Slli => {
                rd_val = rs1_val.wrapping_shl(imm);
            }

            RV32IAL::Srli => {
                rd_val = rs1_val.wrapping_shr(imm);
            }

            RV32IAL::Srai => {
                rd_val = (rs1_val as i32).wrapping_shr(imm) as u32;
            }

            RV32IAL::Add => {
                rd_val = rs1_val.wrapping_add(rs2_val);
            }

            RV32IAL::Sub => {
                rd_val = rs1_val.wrapping_sub(rs2_val);
            }

            RV32IAL::Xor => {
                rd_val = rs1_val ^ rs2_val;
            }

            RV32IAL::Or => {
                rd_val = rs1_val | rs2_val;
            }

            RV32IAL::And => {
                rd_val = rs1_val & rs2_val;
            }

            RV32IAL::Slt => {
                rd_val = if (rs1_val as i32) < (rs2_val as i32) { 1 } else { 0 };
            }

            RV32IAL::Sltu => {
                rd_val = if rs1_val < rs2_val { 1 } else { 0 };
            }

            RV32IAL::Sll => {
                rd_val = rs1_val.wrapping_shl(rs2_val & 0x1F);
            }

            RV32IAL::Srl => {
                rd_val = rs1_val.wrapping_shr(rs2_val & 0x1F);
            }

            RV32IAL::Sra => {
                rd_val = (rs1_val as i32).wrapping_shr(rs2_val & 0x1F) as u32;
            }

            RV32IAL::Ecall => {
                rd_addr = 0;
                next_pc = self.states.regfile.read_csr(RvCsrEnum::MTVEC.into())?;
            }

            RV32IAL::Ebreak => {
                trap = Some(Trap::Ebreak);
            }

            RV32IAL::Fence => {
                rd_addr = 0;
                // Do nothing for now
            }
        }

        Ok(ToWbStage { pc, next_pc, gpr_wmsg: (rd_addr, rd_val), csr_wmsg, trap})
    }

    fn arithmetic_logic_rv32m(&self, pc: u32, inst: RV32M, msg: InstMsg) -> ProcessResult<ToWbStage> {
        let next_pc = pc.wrapping_add(4);
        let rs1_val = msg.rs1;
        let rs2_val = msg.rs2;
        let rd_addr = msg.rd_addr;
        let rd_val;
        let csr_wmsg = (false, 0, 0);
        let trap = None;

        match inst {
            RV32M::Mul => {
                rd_val = rs1_val.wrapping_mul(rs2_val);
            }

            RV32M::Mulh => {
                rd_val = (rs1_val as i64).wrapping_mul(rs2_val as i64).wrapping_shr(32) as u32;
            }

            RV32M::Mulhsu => {
                rd_val = (rs1_val as i32 as i64).wrapping_mul(rs2_val as u32 as i64).wrapping_shr(32) as u32;
            }

            RV32M::Mulhu => {
                rd_val = (rs1_val as u64).wrapping_mul(rs2_val as u64).wrapping_shr(32) as u32;
            }

            RV32M::Div => {
                if rs2_val == 0 {
                    rd_val = 0xFFFFFFFF;
                } else {
                    rd_val = (rs1_val as i32).wrapping_div(rs2_val as i32) as u32;
                }
            }

            RV32M::Divu => {
                if rs2_val == 0 {
                    rd_val = 0xFFFFFFFF;
                } else {
                    rd_val = rs1_val.wrapping_div(rs2_val);
                }
            }

            RV32M::Rem => {
                if rs2_val == 0 {
                    rd_val = rs1_val;
                } else {
                    rd_val = (rs1_val as i32).wrapping_rem(rs2_val as i32) as u32;
                }
            }

            RV32M::Remu => {
                if rs2_val == 0 {
                    rd_val = rs1_val;
                } else {
                    rd_val = rs1_val.wrapping_rem(rs2_val);
                }
            }
        };

        Ok(ToWbStage { pc, next_pc, gpr_wmsg: (rd_addr, rd_val), csr_wmsg, trap})

    }

    pub fn arithmetic_logic_rv32(&self, stage: ToAlStage) -> ProcessResult<ToWbStage> {
        match stage.inst {
            AlInst::RV32I(inst) => self.arithmetic_logic_rv32i(stage.pc, inst, stage.msg),
            AlInst::RV32M(inst) => self.arithmetic_logic_rv32m(stage.pc, inst, stage.msg),
        }
    }
}
