use logger::Logger;
use remu_macro::{log_err, log_todo};
use remu_utils::{ProcessError, ProcessResult};

use crate::emu::Emu;

use super::{InstMsg, InstPattern, RISCV, RV32I};

use state::{mmu::Mask, reg::{riscv::RvCsrEnum, RegfileIo}};

impl Emu {
    fn rv32_i_execute(&mut self, name: RV32I, mut msg: InstMsg) -> ProcessResult<()> {
        let regfile = &mut self.states.regfile;
        let rs1: u32 = regfile.read_gpr(msg.rs1.into()).map_err(|_| ProcessError::Recoverable)?;
        let rs2: u32 = regfile.read_gpr(msg.rs2.into()).map_err(|_| ProcessError::Recoverable)?;
        
        let mut rd_val: u32 = 0;

        let pc: u32 = regfile.read_pc();
        let mut next_pc = pc.wrapping_add(4);

        let imm: u32 = msg.imm;

        let mmu = &mut self.states.mmu;

        match name {
            RV32I::Lui => {
                rd_val = imm;
            }

            RV32I::Auipc => {
                rd_val = pc.wrapping_add(imm);
            }

            RV32I::Jal => {
                rd_val = next_pc;
                next_pc = pc.wrapping_add(imm);
            }

            RV32I::Jalr => {
                rd_val = next_pc;
                next_pc = rs1.wrapping_add(imm);
            }

            RV32I::Beq => {
                msg.rd = 0;
                if rs1 == rs2 {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32I::Bne => {
                msg.rd = 0;
                if rs1 != rs2 {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32I::Blt => {
                msg.rd = 0;
                if (rs1 as i32) < (rs2 as i32) {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32I::Bge => {
                msg.rd = 0;
                if (rs1 as i32) >= (rs2 as i32) {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32I::Bltu => {
                msg.rd = 0;
                if rs1 < rs2 {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32I::Bgeu => {
                msg.rd = 0;
                if rs1 >= rs2 {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32I::Lb => {
                let addr = rs1.wrapping_add(imm);
                rd_val = log_err!(mmu.read(addr, Mask::Byte), ProcessError::Recoverable)? as i8 as u32;
            }

            RV32I::Lh => {
                let addr = rs1.wrapping_add(imm);
                rd_val = log_err!(mmu.read(addr, Mask::Half), ProcessError::Recoverable)? as i16 as u32;
            }

            RV32I::Lw => {
                let addr = rs1.wrapping_add(imm);
                rd_val = log_err!(mmu.read(addr, Mask::Word), ProcessError::Recoverable)?;
            }

            RV32I::Lbu => {
                let addr = rs1.wrapping_add(imm);
                rd_val = log_err!(mmu.read(addr, Mask::Byte), ProcessError::Recoverable)?;
            }

            RV32I::Lhu => {
                let addr = rs1.wrapping_add(imm);
                rd_val = log_err!(mmu.read(addr, Mask::Half), ProcessError::Recoverable)?;
            }

            RV32I::Sb => {
                msg.rd = 0;
                let addr = rs1.wrapping_add(imm);
                log_err!(mmu.write(addr, rs2, Mask::Byte), ProcessError::Recoverable)?;
            }

            RV32I::Sh => {
                msg.rd = 0;
                let addr = rs1.wrapping_add(imm);
                log_err!(mmu.write(addr, rs2, Mask::Half), ProcessError::Recoverable)?;
            }

            RV32I::Sw => {
                msg.rd = 0;
                let addr = rs1.wrapping_add(imm);
                log_err!(mmu.write(addr, rs2, Mask::Word), ProcessError::Recoverable)?;
            }

            RV32I::Addi => {
                rd_val = rs1.wrapping_add(imm);
            }

            RV32I::Slti => {
                rd_val = if (rs1 as i32) < (imm as i32) { 1 } else { 0 };
            }

            RV32I::Sltiu => {
                rd_val = if rs1 < imm { 1 } else { 0 };
            }

            RV32I::Xori => {
                rd_val = rs1 ^ imm;
            }

            RV32I::Ori => {
                rd_val = rs1 | imm;
            }

            RV32I::Andi => {
                rd_val = rs1 & imm;
            }

            RV32I::Slli => {
                rd_val = rs1 << imm;
            }

            RV32I::Srli => {
                rd_val = rs1 >> imm;
            }

            RV32I::Srai => {
                rd_val = (rs1 as i32 >> imm) as u32;
            }

            RV32I::Add => {
                rd_val = rs1.wrapping_add(rs2);
            }

            RV32I::Sub => {
                rd_val = rs1.wrapping_sub(rs2);
            }

            RV32I::Xor => {
                rd_val = rs1 ^ rs2;
            }

            RV32I::Or => {
                rd_val = rs1 | rs2;
            }

            RV32I::And => {
                rd_val = rs1 & rs2;
            }

            RV32I::Slt => {
                rd_val = if (rs1 as i32) < (rs2 as i32) { 1 } else { 0 };
            }

            RV32I::Sltu => {
                rd_val = if rs1 < rs2 { 1 } else { 0 };
            }

            RV32I::Sll => {
                rd_val = rs1 << (rs2 & 0x1F);
            }

            RV32I::Srl => {
                rd_val = rs1 >> (rs2 & 0x1F);
            }

            RV32I::Sra => {
                rd_val = (rs1 as i32 >> (rs2 & 0x1F)) as u32;
            }

            RV32I::Ecall => {
                msg.rd = 0;
                regfile.write_csr(RvCsrEnum::MEPC.into(), pc).map_err(|_| ProcessError::Recoverable)?;
                regfile.write_csr(RvCsrEnum::MCAUSE.into(), 0x0000000b).map_err(|_| ProcessError::Recoverable)?;
                next_pc = regfile.read_csr(RvCsrEnum::MTVEC.into()).map_err(|_| ProcessError::Recoverable)?;
            }

            RV32I::Ebreak => {
                let a0 = regfile.read_gpr(10).unwrap();
                (self.callback.trap)(a0 == 0);
                return Err(ProcessError::Recoverable);
            }

            RV32I::Fence => {
                msg.rd = 0;
                // Do nothing
            }
        }
        
        regfile.write_gpr(msg.rd.into(), rd_val).map_err(|_| ProcessError::Recoverable)?;

        regfile.write_pc(next_pc);

        Ok(())
    }

    pub fn execute(&mut self, inst: InstPattern) -> ProcessResult<()> {
        let belongs_to = inst.name;
        if !self.instruction_set.enable(belongs_to) {
            return Err(ProcessError::Recoverable)
        }

        match belongs_to {
            RISCV::RV32I(name) => {
                self.rv32_i_execute(name, inst.msg)?;
            }

            RISCV::RV32M(_) => {
                log_todo!();
            }

            RISCV::Priv(_) => {
                log_todo!();
            }

            RISCV::Zicsr(_) => {
                log_todo!();
            }
        }

        Ok(())
    }
}