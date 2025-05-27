use remu_utils::ProcessResult;
use state::reg::riscv::Trap;

use crate::emu::{isa::riscv::{backend::{AlCtrl, LsCtrl, ToAlStage, ToLsStage, WbCtrl}}, Emu};

#[derive(Default, Clone, Copy)]
pub struct ToIsStage {
    pub pc: u32,

    pub rs1_val: u32,
    pub rs2_val: u32,
    pub gpr_waddr: u8,
    pub imm: u32,

    pub inst_type: InstType,
    pub is_ctrl: IsCtrl,

    pub al_ctrl: AlCtrl,
    pub ls_ctrl: LsCtrl,

    pub wb_ctrl: WbCtrl,

    pub trap: Option<Trap>,
}

#[derive(Default, Clone, Copy)]
pub enum IsLogic {
    #[default]
    EQ,
    NE,

    LT,
    GE,

    LTU,
    GEU,

    SLTI,
    SLTIU,
}

#[derive(Default, Clone, Copy)]
pub enum SRCA {
    #[default]
    RS1,
    ZERO,
    PC,
}

#[derive(Default, Clone, Copy)]
pub enum SRCB {
    #[default]
    RS2,
    IMM,
    LogicBranch,
    LogicSet,
}

#[derive(Default, Clone, Copy)]
pub struct IsCtrl {
    pub srca: SRCA,
    pub srcb: SRCB,
    pub logic: IsLogic,
}

#[derive(Default, Clone, Copy)]
pub enum InstType {
    #[default]
    AL,
    LS,
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
        let rs1_val = stage.rs1_val;
        let rs2_val: u32 = stage.rs2_val;
        let gpr_waddr = stage.gpr_waddr;
        let imm = stage.imm;

        let inst_type = stage.inst_type;

        let pc = stage.pc;
        let wb_ctrl = stage.wb_ctrl;

        match inst_type {
            InstType::AL => {
                let al_ctrl = stage.al_ctrl;

                let logic = match stage.is_ctrl.logic {
                    IsLogic::EQ => rs1_val == rs2_val,
                    IsLogic::NE => rs1_val != rs2_val,
                    IsLogic::LT => (rs1_val as i32) < (rs2_val as i32),
                    IsLogic::GE => (rs1_val as i32) >= (rs2_val as i32),
                    IsLogic::LTU => rs1_val < rs2_val,
                    IsLogic::GEU => rs1_val >= rs2_val,
                    IsLogic::SLTI => (rs1_val as i32) < (imm as i32),
                    IsLogic::SLTIU => rs1_val < imm,
                };
        
                let srca = match stage.is_ctrl.srca {
                    SRCA::RS1 => rs1_val,
                    SRCA::ZERO => 0,
                    SRCA::PC => pc,
                };
        
                let srcb = match stage.is_ctrl.srcb {
                    SRCB::RS2 => rs2_val,
                    SRCB::IMM => imm,
                    SRCB::LogicBranch => if logic { imm } else { 4 },
                    SRCB::LogicSet => if logic { 1 } else { 0 },
                };

                let trap = stage.trap;
        
                Ok(IsOutStage::AL(ToAlStage { pc, srca, srcb, ctrl: al_ctrl, wb_ctrl, gpr_waddr, trap }))
            }

            InstType::LS => {
                let ls_ctrl = stage.ls_ctrl;

                Ok(IsOutStage::LS(ToLsStage {
                    pc, 
                    ls_ctrl,
                    gpr_waddr,

                    addr: rs1_val.wrapping_add(imm),
                    data: rs2_val,
                }))
            }
        }
    }
}
