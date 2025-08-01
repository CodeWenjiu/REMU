use remu_utils::ProcessResult;
use state::cache::BRMsg;

use crate::emu::isa::riscv::BasicStageMsg;
use crate::emu::EmuHardware;

use super::{ToWbStage, WbCtrl, };
use super::super::super::instruction::{RV32IAL, RV32M};

pub enum AlInst {
    RV32I(RV32IAL),
    RV32M(RV32M),
}

impl Default for AlInst {
    fn default() -> Self {
        AlInst::RV32I(RV32IAL::default())
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub enum AlCtrl {
    #[default]
    DontCare,

    B,

    Add,   
    Sub,     
    
    And,   
    Or,    
    Xor,   
    
    Sll,   
    Srl,   
    Sra,   
    
    Mul,
    Mulh,
    Mulhsu,
    Mulhu,
    Div,
    Divu,
    Rem,
    Remu,
}

#[derive(Default, Clone, Debug)]
pub struct ToAlStage {
    pub msg: BasicStageMsg,

    pub srca: u32,
    pub srcb: u32,

    pub al_ctrl: AlCtrl,
    pub wb_ctrl: WbCtrl,

    pub br: BRMsg,

    pub gpr_waddr: u8,
    pub csr_waddr: u16,
}

impl EmuHardware {
    pub fn arithmetic_logic_rv32(&self, stage: ToAlStage) -> ProcessResult<ToWbStage> {
        let msg = stage.msg;

        let mut result =  0;
        let srca = stage.srca;
        let srcb = stage.srcb;

        let gpr_waddr = stage.gpr_waddr;
        let csr_waddr = stage.csr_waddr;

        let wb_ctrl = stage.wb_ctrl;

        if msg.trap == None {
            match stage.al_ctrl {
                AlCtrl::B => {
                    result = srcb;
                }

                AlCtrl::Add => {
                    result = srca.wrapping_add(srcb);
                }

                AlCtrl::Sub => {
                    result = srca.wrapping_sub(srcb);
                }

                AlCtrl::And => {
                    result = srca & srcb;
                }

                AlCtrl::Or => {
                    result = srca | srcb;
                }

                AlCtrl::Xor => {
                    result = srca ^ srcb;
                }

                AlCtrl::Sll => {
                    result = srca.wrapping_shl(srcb & 0x1F);
                }

                AlCtrl::Srl => {
                    result = srca.wrapping_shr(srcb & 0x1F);
                }

                AlCtrl::Sra => {
                    result = (srca as i32).wrapping_shr(srcb & 0x1F) as u32;
                }

                AlCtrl::Mul => {
                    result = srca.wrapping_mul(srcb);
                }

                AlCtrl::Mulh => {
                    result = (srca as i64).wrapping_mul(srcb as i64).wrapping_shr(32) as u32;
                }

                AlCtrl::Mulhsu => {
                    result = (srca as i32 as i64).wrapping_mul(srcb as u32 as i64).wrapping_shr(32) as u32;
                }

                AlCtrl::Mulhu => {
                    result = (srca as u64).wrapping_mul(srcb as u64).wrapping_shr(32) as u32;
                }

                AlCtrl::Div => {
                    if srcb == 0 {
                        result = 0xFFFFFFFF;
                    } else {
                        result = (srca as i32).wrapping_div(srcb as i32) as u32;
                    }
                }

                AlCtrl::Divu => {
                    if srcb == 0 {
                        result = 0xFFFFFFFF;
                    } else {
                        result = srca.wrapping_div(srcb);
                    }
                }

                AlCtrl::Rem => {
                    if srcb == 0 {
                        result = srca;
                    } else {
                        result = (srca as i32).wrapping_rem(srcb as i32) as u32;
                    }
                }

                AlCtrl::Remu => {
                    if srcb == 0 {
                        result = srca;
                    } else {
                        result = srca.wrapping_rem(srcb);
                    }
                }

                AlCtrl::DontCare => {
                    // ALU will not change state, so we can ignore it
                },
            };
        }

        Ok(ToWbStage { msg, result, csr_rdata: srca, br: stage.br, gpr_waddr, csr_waddr, wb_ctrl })
    }
}
