use logger::Logger;
use remu_macro::{log_err, log_error, log_todo};
use remu_utils::{ProcessError, ProcessResult};

use crate::emu::Emu;

use super::{InstMsg, InstPattern, ToAlStage, ToIdStage, RISCV, RV32I, RV32IAL, RV32ILS, RV32M};

use state::{mmu::Mask, model::BasePipeCell, reg::{riscv::RvCsrEnum, RegfileIo}};


#[derive(Default)]
struct ToAgStage {
    pub pc: u32,
    pub inst: RV32ILS,
    pub msg: InstMsg, 
}

#[derive(Default)]
pub struct ToLsStage {
    pub pc: u32,
    pub to_ls: RV32ILS,
    pub inst_msg: InstMsg,
}

#[derive(Default)]
pub struct ToWbStage {
    pub pc: u32,
    pub next_pc: u32,
    pub gpr_wmsg: (u8, u32),
    pub csr_wmsg: [(bool, u32, u32); 2],
}

#[derive(Default)]
pub struct EmuPipeCell {
    to_id: ToIdStage,
    to_al: ToAlStage,
    to_ag: ToAgStage,
    to_ls: ToLsStage,
    to_wb: ToWbStage,
}

impl Emu {
    fn rv32_i_al_execute(&mut self, name: RV32IAL, mut msg: InstMsg) -> ProcessResult<u32> {
        let regfile = &mut self.states.regfile;
        let rs1: u32 = msg.rs1;
        let rs2: u32 = msg.rs2;
        
        let mut rd_val: u32 = 0;

        let pc: u32 = regfile.read_pc();
        let mut next_pc = pc.wrapping_add(4);

        let imm: u32 = msg.imm;

        match name {
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
                next_pc = rs1.wrapping_add(imm);
            }

            RV32IAL::Beq => {
                msg.rd_addr = 0;
                if rs1 == rs2 {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32IAL::Bne => {
                msg.rd_addr = 0;
                if rs1 != rs2 {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32IAL::Blt => {
                msg.rd_addr = 0;
                if (rs1 as i32) < (rs2 as i32) {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32IAL::Bge => {
                msg.rd_addr = 0;
                if (rs1 as i32) >= (rs2 as i32) {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32IAL::Bltu => {
                msg.rd_addr = 0;
                if rs1 < rs2 {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32IAL::Bgeu => {
                msg.rd_addr = 0;
                if rs1 >= rs2 {
                    next_pc = pc.wrapping_add(imm);
                }
            }

            RV32IAL::Addi => {
                rd_val = rs1.wrapping_add(imm);
            }

            RV32IAL::Slti => {
                rd_val = if (rs1 as i32) < (imm as i32) { 1 } else { 0 };
            }

            RV32IAL::Sltiu => {
                rd_val = if rs1 < imm { 1 } else { 0 };
            }

            RV32IAL::Xori => {
                rd_val = rs1 ^ imm;
            }

            RV32IAL::Ori => {
                rd_val = rs1 | imm;
            }

            RV32IAL::Andi => {
                rd_val = rs1 & imm;
            }

            RV32IAL::Slli => {
                rd_val = rs1.wrapping_shl(imm);
            }

            RV32IAL::Srli => {
                rd_val = rs1.wrapping_shr(imm);
            }

            RV32IAL::Srai => {
                rd_val = (rs1 as i32).wrapping_shr(imm) as u32;
            }

            RV32IAL::Add => {
                rd_val = rs1.wrapping_add(rs2);
            }

            RV32IAL::Sub => {
                rd_val = rs1.wrapping_sub(rs2);
            }

            RV32IAL::Xor => {
                rd_val = rs1 ^ rs2;
            }

            RV32IAL::Or => {
                rd_val = rs1 | rs2;
            }

            RV32IAL::And => {
                rd_val = rs1 & rs2;
            }

            RV32IAL::Slt => {
                rd_val = if (rs1 as i32) < (rs2 as i32) { 1 } else { 0 };
            }

            RV32IAL::Sltu => {
                rd_val = if rs1 < rs2 { 1 } else { 0 };
            }

            RV32IAL::Sll => {
                rd_val = rs1.wrapping_shl(rs2 & 0x1F);
            }

            RV32IAL::Srl => {
                rd_val = rs1.wrapping_shr(rs2 & 0x1F);
            }

            RV32IAL::Sra => {
                rd_val = (rs1 as i32).wrapping_shr(rs2 & 0x1F) as u32;
            }

            RV32IAL::Ecall => {
                msg.rd_addr = 0;
                regfile.write_csr(RvCsrEnum::MEPC.into(), pc)?;
                regfile.write_csr(RvCsrEnum::MCAUSE.into(), 0x0000000b)?;
                next_pc = regfile.read_csr(RvCsrEnum::MTVEC.into())?;
            }

            RV32IAL::Ebreak => {
                let a0 = regfile.read_gpr(10).unwrap();
                (self.callback.trap)(a0 == 0);
                return Err(ProcessError::Recoverable);
            }

            RV32IAL::Fence => {
                msg.rd_addr = 0;
                // Do nothing
            }
        }
        
        regfile.write_gpr(msg.rd_addr.into(), rd_val)?;

        regfile.write_pc(next_pc);

        Ok(next_pc)
    }

    fn rv32_i_ls_execute(&mut self, name: RV32ILS, mut msg: InstMsg) -> ProcessResult<u32> {
        let regfile = &mut self.states.regfile;
        let rs1: u32 = msg.rs1;
        let rs2: u32 = msg.rs2;
        
        let mut rd_val: u32 = 0;

        let pc: u32 = regfile.read_pc();
        let next_pc = pc.wrapping_add(4);

        let imm: u32 = msg.imm;

        let mmu = &mut self.states.mmu;

        match name {
            RV32ILS::Lb => {
                let addr = rs1.wrapping_add(imm);
                let data = log_err!(mmu.read(addr, Mask::Byte), ProcessError::Recoverable)?;
                rd_val = data.1 as i8 as u32;
                if data.0 == true {
                    (self.callback.difftest_skip)();
                }
            }

            RV32ILS::Lh => {
                let addr = rs1.wrapping_add(imm);
                let data = log_err!(mmu.read(addr, Mask::Half), ProcessError::Recoverable)?;
                rd_val = data.1 as i16 as u32;
                if data.0 == true {
                    (self.callback.difftest_skip)();
                }
            }

            RV32ILS::Lw => {
                let addr = rs1.wrapping_add(imm);
                let data = log_err!(mmu.read(addr, Mask::Word), ProcessError::Recoverable)?;
                rd_val = data.1;
                if data.0 == true {
                    (self.callback.difftest_skip)();
                }
            }

            RV32ILS::Lbu => {
                let addr = rs1.wrapping_add(imm);
                let data = log_err!(mmu.read(addr, Mask::Byte), ProcessError::Recoverable)?;
                rd_val = data.1;
                if data.0 == true {
                    (self.callback.difftest_skip)();
                }
            }

            RV32ILS::Lhu => {
                let addr = rs1.wrapping_add(imm);
                let data = log_err!(mmu.read(addr, Mask::Half), ProcessError::Recoverable)?;
                rd_val = data.1;
                if data.0 == true {
                    (self.callback.difftest_skip)();
                }
            }

            RV32ILS::Sb => {
                msg.rd_addr = 0;
                let addr = rs1.wrapping_add(imm);
                if log_err!(mmu.write(addr, rs2, Mask::Byte), ProcessError::Recoverable)? == true {
                    (self.callback.difftest_skip)();
                }
            }

            RV32ILS::Sh => {
                msg.rd_addr = 0;
                let addr = rs1.wrapping_add(imm);
                if log_err!(mmu.write(addr, rs2, Mask::Half), ProcessError::Recoverable)? == true {
                    (self.callback.difftest_skip)();
                }
            }

            RV32ILS::Sw => {
                msg.rd_addr = 0;
                let addr = rs1.wrapping_add(imm);
                if log_err!(mmu.write(addr, rs2, Mask::Word), ProcessError::Recoverable)? == true {
                    (self.callback.difftest_skip)();
                }
            }
        }
            
        regfile.write_gpr(msg.rd_addr.into(), rd_val)?;

        regfile.write_pc(next_pc);

        Ok(next_pc)
    }

    fn rv32_i_execute(&mut self, name: RV32I, msg: InstMsg) -> ProcessResult<u32> {
        match name {
            RV32I::AL(name) => self.rv32_i_al_execute(name, msg),
            RV32I::LS(name) => self.rv32_i_ls_execute(name, msg),
        }
    }

    fn rv32_e_execute(&mut self, name: RV32I, mut msg: InstMsg) -> ProcessResult<u32> {
        msg.rd_addr &= 0xF;
        self.rv32_i_execute(name, msg)
    }

    fn rv32_m_execute(&mut self, _name: RV32M, msg: InstMsg) -> ProcessResult<u32> {
        let regfile = &mut self.states.regfile;
        let rs1: u32 = msg.rs1;
        let rs2: u32 = msg.rs2;
        
        let rd_val: u32;

        let pc: u32 = regfile.read_pc();
        let next_pc = pc.wrapping_add(4);

        match _name {
            RV32M::Mul => {
                rd_val = rs1.wrapping_mul(rs2);
            }

            RV32M::Mulh => {
                rd_val = (rs1 as i64).wrapping_mul(rs2 as i64).wrapping_shr(32) as u32;
            }

            RV32M::Mulhsu => {
                rd_val = (rs1 as i32 as i64).wrapping_mul(rs2 as u32 as i64).wrapping_shr(32) as u32;
            }

            RV32M::Mulhu => {
                rd_val = (rs1 as u64).wrapping_mul(rs2 as u64).wrapping_shr(32) as u32;
            }

            RV32M::Div => {
                if rs2 == 0 {
                    rd_val = 0xFFFFFFFF;
                } else {
                    rd_val = (rs1 as i32).wrapping_div(rs2 as i32) as u32;
                }
            }

            RV32M::Divu => {
                if rs2 == 0 {
                    rd_val = 0xFFFFFFFF;
                } else {
                    rd_val = rs1.wrapping_div(rs2);
                }
            }

            RV32M::Rem => {
                if rs2 == 0 {
                    rd_val = rs1;
                } else {
                    rd_val = (rs1 as i32).wrapping_rem(rs2 as i32) as u32;
                }
            }

            RV32M::Remu => {
                if rs2 == 0 {
                    rd_val = rs1;
                } else {
                    rd_val = rs1.wrapping_rem(rs2);
                }
            }
        }
        
        regfile.write_gpr(msg.rd_addr.into(), rd_val).map_err(|_| ProcessError::Recoverable)?;

        regfile.write_pc(next_pc);

        Ok(next_pc)
    }
    
    pub fn self_if_catch(&mut self) -> ProcessResult<()> {
        let pc = self.states.regfile.read_pc();
        let inst = log_err!(
            self.states.mmu.read(pc, state::mmu::Mask::Word),
            ProcessError::Recoverable
        )?.1;

        self.pipe.to_id = ToIdStage {
            pc,
            inst,
        };

        self.states.pipe_state.send((pc, inst), BasePipeCell::IDU)?;

        Ok(())
    }

    pub fn self_id_catch(&mut self, fetched_pc: u32) -> ProcessResult<()> {
        let pc = self.pipe.to_id.pc;
        let inst = self.pipe.to_id.inst;

        let inst_pattern = self.decode(ToIdStage { pc: pc, inst: inst })?;
        
        self.pipe.to_al.pc = pc;
        self.pipe.to_al.msg = inst_pattern.msg;

        match inst_pattern.name {
            RISCV::RV32I(RV32I::LS(name)) => {
                self.states.pipe_state.send((pc, inst), BasePipeCell::AGU)?;
                self.pipe.to_ag.inst = name;
            }

            RISCV::RV32I(RV32I::AL(name)) => {
                self.states.pipe_state.send((pc, inst), BasePipeCell::ALU)?;
                self.pipe.to_al.inst = name;
            }

            _ => unreachable!(),
        }


        if fetched_pc != pc {
            log_error!(format!("IDU catch PC mismatch: fetched {:#08x}, expected {:#08x}", fetched_pc, pc));
            return Err(ProcessError::Recoverable);
        }

        Ok(())
    }

    pub fn al_execute(&mut self, fetched_pc: u32) -> ProcessResult<()> {
        let pc = self.pipe.to_al.pc;
        let inst = self.pipe.to_al.inst;
        let msg = self.pipe.to_al.msg.clone();

        self.rv32_i_al_execute(inst, msg)?;

        if fetched_pc != pc {
            log_error!(format!("ALU catch PC mismatch: fetched {:#08x}, expected {:#08x}", fetched_pc, pc));
            return Err(ProcessError::Recoverable);
        }
        
        Ok(())
    }

    pub fn ag_execute(&mut self, fetched_pc: u32) -> ProcessResult<()> {
        let pc = self.pipe.to_al.pc;
        let inst = self.pipe.to_ag.inst;
        let msg = self.pipe.to_al.msg.clone();

        Ok(())
    }

    pub fn execute(&mut self, inst: InstPattern) -> ProcessResult<u32> {
        let belongs_to = inst.name;
        if !self.instruction_set.enable(belongs_to) {
            return Err(ProcessError::Recoverable)
        }

        match belongs_to {
            RISCV::RV32I(name) => {
                return self.rv32_i_execute(name, inst.msg);
            }

            RISCV::RV32E(name) => {
                return self.rv32_e_execute(name, inst.msg);
            }

            RISCV::RV32M(name) => {
                return self.rv32_m_execute(name, inst.msg);
            }

            RISCV::Priv(_) => {
                log_todo!();
            }

            RISCV::Zicsr(_) => {
                log_todo!();
            }
        }

        Err(ProcessError::Recoverable)
    }
}