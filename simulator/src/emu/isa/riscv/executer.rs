use logger::Logger;
use remu_macro::log_err;

use crate::emu::Emu;

use super::{InstMsg, RV32I};

use state::mmu::Mask;

impl Emu {
    fn rv32_i_execute(&mut self, name: RV32I, msg: InstMsg) -> Result<(), ()> {
        let regfile = &mut self.states.borrow_mut().regfile;
        let rs1: u32 = regfile.read_gpr(msg.rs1.into()).map_err(|_| {
            Logger::show("Error reading register", Logger::ERROR);
        })?;
        let rs2: u32 = regfile.read_gpr(msg.rs2.into()).map_err(|_| {
            Logger::show("Error reading register", Logger::ERROR);
        })?;
        
        let mut rd_val: u32 = 0;

        let pc: u32 = regfile.read_pc();
        let mut next_pc = pc.wrapping_add(4);

        let imm: u32 = msg.imm;

        let mmu = &mut self.states.borrow_mut().mmu;

        match name {
            RV32I::Lui => {
                rd_val = imm;
            }

            RV32I::Auipc => {
                rd_val = rs1.wrapping_add(pc);
            }

            RV32I::Jal => {
                rd_val = next_pc;
            }

            RV32I::Jalr => {
                rd_val = next_pc;
                next_pc = rs1.wrapping_add(imm);
            }

            RV32I::Beq => {
                if rs1 == rs2 {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32I::Bne => {
                if rs1 != rs2 {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32I::Blt => {
                if (rs1 as i32) < (rs2 as i32) {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32I::Bge => {
                if (rs1 as i32) >= (rs2 as i32) {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32I::Bltu => {
                if rs1 < rs2 {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32I::Bgeu => {
                if rs1 >= rs2 {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32I::Lb => {
                let addr = rs1.wrapping_add(imm);
                rd_val = log_err!(mmu.read(addr, Mask::Byte))? as i8 as u32;
            }

            RV32I::Add => {
                rd_val = rs1.wrapping_add(rs2);
            }

            RV32I::Addi => {
                rd_val = rs1.wrapping_add(imm);
            }

            _ => {
                Logger::todo();
            }
        }
        
        regfile.write_gpr(msg.rd.into(), rd_val).map_err(|_| {
            Logger::show("Error writing register", Logger::ERROR);
        })?;

        regfile.write_pc(next_pc);

        Ok(())
    }
}