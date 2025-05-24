use remu_utils::ProcessResult;

use crate::emu::{isa::riscv::{backend::{AlCtrl, ToAlStage, ToLsStage, WbCtrl}, Trap, RISCV, RV32I, RV32IAL, RV32M}, Emu};



#[derive(Default, Clone, Copy)]
pub struct ToIsStage {
    pub pc: u32,
    pub inst: RISCV,
    pub rs1: u32,
    pub rs2: u32,
    pub rd_addr: u8,
    pub imm: u32,
}

pub enum IsOutStage {
    AL(ToAlStage),
    LS(ToLsStage),
}

impl Default for IsOutStage {
    fn default() -> Self {
        Self::AL(ToAlStage::default())
    }
}

impl Emu {
    
    pub fn instruction_issue(&mut self, stage: ToIsStage) -> ProcessResult<IsOutStage> {
        let rs1_val = stage.rs1;
        let rs2_val: u32 = stage.rs2;
        let mut gpr_waddr = stage.rd_addr;
        let imm = stage.imm;

        let inst = stage.inst;

        let pc = stage.pc;
        let mut srca = rs1_val;
        let mut srcb = rs2_val;
        let mut ctrl = AlCtrl::Add;
        let mut wb_ctrl = WbCtrl::WriteGpr;

        let mut trap = None;

        match inst {
            RISCV::RV32I(RV32I::AL(inst)) => {
                match inst {
                    RV32IAL::Lui => {
                        srca = imm;
                        srcb = 0;
                    }

                    RV32IAL::Auipc => {
                        srca = pc;
                        srcb = imm;
                    }

                    RV32IAL::Jal => {
                        wb_ctrl = WbCtrl::Jump;
                        srca = pc;
                        srcb = imm;
                    }

                    RV32IAL::Jalr => {
                        wb_ctrl = WbCtrl::Jump;
                        srcb = imm;
                    }

                    // logic work should move to IS stage in the future
                    RV32IAL::Beq => {
                        wb_ctrl = WbCtrl::Jump;
                        gpr_waddr = 0; // there is no rd need to link address to register
                        srca = pc;
                        srcb = if rs1_val == rs2_val { imm } else { 4 };
                    }

                    RV32IAL::Bne => {
                        wb_ctrl = WbCtrl::Jump;
                        gpr_waddr = 0; 
                        srca = pc;
                        srcb = if rs1_val != rs2_val { imm } else { 4 };
                    }

                    RV32IAL::Blt => {
                        wb_ctrl = WbCtrl::Jump;
                        gpr_waddr = 0; 
                        srca = pc;
                        srcb = if (rs1_val as i32) < (rs2_val as i32) { imm } else { 4 };
                    }

                    RV32IAL::Bge => {
                        wb_ctrl = WbCtrl::Jump;
                        gpr_waddr = 0; 
                        srca = pc;
                        srcb = if (rs1_val as i32) >= (rs2_val as i32) { imm } else { 4 };
                    }

                    RV32IAL::Bltu => {
                        wb_ctrl = WbCtrl::Jump;
                        gpr_waddr = 0; 
                        srca = pc;
                        srcb = if rs1_val < rs2_val { imm } else { 4 };
                    }

                    RV32IAL::Bgeu => {
                        wb_ctrl = WbCtrl::Jump;
                        gpr_waddr = 0; 
                        srca = pc;
                        srcb = if rs1_val >= rs2_val { imm } else { 4 };
                    }

                    RV32IAL::Addi => {
                        srcb = imm;
                    }

                    RV32IAL::Slti => {
                        srca = if (rs1_val as i32) < (imm as i32) { 1 } else { 0 };
                        srcb = 0;
                    }

                    RV32IAL::Sltiu => {
                        srca = if rs1_val < imm { 1 } else { 0 };
                        srcb = 0;
                    }

                    RV32IAL::Xori => {
                        ctrl = AlCtrl::Xor;
                        srcb = imm;
                    }

                    RV32IAL::Ori => {
                        ctrl = AlCtrl::Or;
                        srcb = imm;
                    }

                    RV32IAL::Andi => {
                        ctrl = AlCtrl::And;
                        srcb = imm;
                    }

                    RV32IAL::Slli => {
                        ctrl = AlCtrl::Sll;
                        srcb = imm;
                    }

                    RV32IAL::Srli => {
                        ctrl = AlCtrl::Srl;
                        srcb = imm;
                    }

                    RV32IAL::Srai => {
                        ctrl = AlCtrl::Sra;
                        srcb = imm;
                    }

                    RV32IAL::Add => {}

                    RV32IAL::Sub => {
                        ctrl = AlCtrl::Sub;
                    }

                    RV32IAL::Xor => {
                        ctrl = AlCtrl::Xor;
                    }

                    RV32IAL::Or => {
                        ctrl = AlCtrl::Or;
                    }

                    RV32IAL::And => {
                        ctrl = AlCtrl::And;
                    }

                    RV32IAL::Slt => {
                        srca = if (rs1_val as i32) < (rs2_val as i32) { 1 } else { 0 };
                        srcb = 0;
                    }

                    RV32IAL::Sltu => {
                        srca = if rs1_val < rs2_val { 1 } else { 0 };
                        srcb = 0;
                    }

                    RV32IAL::Sll => {
                        ctrl = AlCtrl::Sll;
                    }

                    RV32IAL::Srl => {
                        ctrl = AlCtrl::Srl;
                    }

                    RV32IAL::Sra => {
                        ctrl = AlCtrl::Sra;
                    }

                    RV32IAL::Ecall => {
                        trap = Some(Trap::EcallM);
                    }
        
                    RV32IAL::Ebreak => {
                        trap = Some(Trap::Ebreak);
                    }
        
                    RV32IAL::Fence => {
                        gpr_waddr = 0; // do nothing for now
                    }
                }
            }

            RISCV::RV32I(RV32I::LS(inst)) => {
                return Ok(IsOutStage::LS(ToLsStage {
                    pc,
                    inst,
                    rd_addr: gpr_waddr,

                    addr: rs1_val.wrapping_add(imm),
                    data: rs2_val,
                }));
            }

            RISCV::RV32M(inst) => {
                match inst {
                    RV32M::Mul => {
                        ctrl = AlCtrl::Mul;
                    }

                    RV32M::Mulh => {
                        ctrl = AlCtrl::Mulh;
                    }

                    RV32M::Mulhsu => {
                        ctrl = AlCtrl::Mulhsu;
                    }

                    RV32M::Mulhu => {
                        ctrl = AlCtrl::Mulhu;
                    }

                    RV32M::Div => {
                        ctrl = AlCtrl::Div;
                    }

                    RV32M::Divu => {
                        ctrl = AlCtrl::Divu;
                    }

                    RV32M::Rem => {
                        ctrl = AlCtrl::Rem;
                    }

                    RV32M::Remu => {
                        ctrl = AlCtrl::Remu;
                    }
                }
            }

            _ => unreachable!()
        };

        Ok(IsOutStage::AL(ToAlStage { pc, srca, srcb, ctrl, wb_ctrl, gpr_waddr, trap }))
    }
}
