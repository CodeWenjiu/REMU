use remu_utils::ProcessResult;
use state::reg::{riscv::RvCsrEnum, RegfileIo};

use crate::emu::Emu;

use super::{InstMsg, ToWbStage, RV32IAL};

#[derive(Default)]
pub struct ToAlStage {
    pub pc: u32,
    pub inst: RV32IAL,
    pub msg: InstMsg, 
}

impl Emu {
    pub fn arithmetic_logic_rv32i(&self, stage: ToAlStage) -> ProcessResult<ToWbStage> {
        let pc = stage.pc;
        let mut next_pc = pc.wrapping_add(4);

        let imm = stage.msg.imm;
        let rs1_val = stage.msg.rs1;
        let rs2_val = stage.msg.rs2;

        let mut rd_addr = stage.msg.rd_addr;
        let mut rd_val = 0;

        let mut csr_msg_a = (false, 0, 0);
        let mut csr_msg_b = (false, 0, 0);

        match stage.inst {
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
                csr_msg_a = (true, RvCsrEnum::MEPC.into(), pc);
                csr_msg_b = (true, RvCsrEnum::MCAUSE.into(), 0x0000000b);
                next_pc = self.states.regfile.read_csr(RvCsrEnum::MTVEC.into())?;
            }

            RV32IAL::Ebreak => {
            }

            _ => unreachable!()
        }

        Ok(ToWbStage { pc, next_pc, gpr_wmsg: (rd_addr, rd_val), csr_wmsg: {[csr_msg_a, csr_msg_b]}})
    }
}
