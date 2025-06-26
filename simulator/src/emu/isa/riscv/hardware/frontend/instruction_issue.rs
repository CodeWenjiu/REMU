use remu_macro::{log_err, log_error};
use remu_utils::{ProcessError, ProcessResult};
use state::reg::{RegfileIo};

use crate::emu::{isa::riscv::{hardware::backend::{AlCtrl, LsCtrl, ToAlStage, ToLsStage, WbCtrl}, BasicStageMsg}, EmuHardware};

#[derive(Default, Clone, Copy, Debug)]
pub struct ToIsStage {
    pub msg: BasicStageMsg,

    pub rs1_addr: u8,
    pub rs1_val: u32,
    pub rs2_addr: u8,
    pub rs2_val: u32,

    pub gpr_waddr: u8,
    pub imm: u32,

    pub is_ctrl: IsCtrl,

    pub al_ctrl: AlCtrl,
    pub ls_ctrl: LsCtrl,

    pub wb_ctrl: WbCtrl,
}

#[derive(Default, Clone, Copy, Debug)]
pub enum InstType {
    #[default]
    AL,
    LS,
}

#[derive(Default, Clone, Copy, Debug)]
pub enum IsLogic {
    #[default]
    DontCare,

    EQ,
    NE,

    LT,
    GE,

    LTU,
    GEU,

    SLTI,
    SLTIU,
}

#[derive(Default, Clone, Copy, Debug)]
pub enum SRCA {
    #[default]
    DontCare,

    RS1,
    PC,
    CSR,
    ZERO,
}

#[derive(Default, Clone, Copy, Debug)]
pub enum SRCB {
    #[default]
    DontCare,

    RS1,
    RS2,
    IMM,
    LogicBranch,
    LogicSet,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct IsCtrl {
    pub inst_type: InstType,
    pub srca: SRCA,
    pub srcb: SRCB,
    pub logic: IsLogic,
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

impl EmuHardware {
    
    pub fn instruction_issue(&mut self, stage: ToIsStage) -> ProcessResult<IsOutStage> {
        let rs1_val = stage.rs1_val;
        let rs2_val: u32 = stage.rs2_val;
        let gpr_waddr = stage.gpr_waddr;
        let imm = stage.imm;

        let inst_type = stage.is_ctrl.inst_type;

        let msg = stage.msg;
        let wb_ctrl = stage.wb_ctrl;

        match inst_type {
            InstType::AL => {
                let al_ctrl = stage.al_ctrl;

                let logic = match stage.is_ctrl.logic {
                    IsLogic::EQ => Some(rs1_val == rs2_val),
                    IsLogic::NE => Some(rs1_val != rs2_val),
                    IsLogic::LT => Some((rs1_val as i32) < (rs2_val as i32)),
                    IsLogic::GE => Some((rs1_val as i32) >= (rs2_val as i32)),
                    IsLogic::LTU => Some(rs1_val < rs2_val),
                    IsLogic::GEU => Some(rs1_val >= rs2_val),
                    IsLogic::SLTI => Some((rs1_val as i32) < (imm as i32)),
                    IsLogic::SLTIU => Some(rs1_val < imm),
                    IsLogic::DontCare => None,
                };
        
                let srca = match stage.is_ctrl.srca {
                    SRCA::RS1 => rs1_val,
                    SRCA::PC => msg.pc,
                    SRCA::CSR => {
                        log_err!(self.states.regfile.read_csr(imm & 0xFFF), ProcessError::Recoverable)?
                    },
                    SRCA::ZERO => 0,
                    SRCA::DontCare => {
                        log_error!(format!("SRCA::DontCare should not be used at pc: {:#08x}", msg.pc));
                        return Err(ProcessError::Recoverable);
                    },
                };
        
                let srcb = match stage.is_ctrl.srcb {
                    SRCB::RS1 => rs1_val,
                    SRCB::RS2 => rs2_val,
                    SRCB::IMM => imm,
                    SRCB::LogicBranch => if logic.unwrap() { imm } else { 4 },
                    SRCB::LogicSet => if logic.unwrap() { 1 } else { 0 },
                    SRCB::DontCare => {
                        log_error!(format!("SRCB::DontCare should not be used at pc: {:#08x}", msg.pc));
                        return Err(ProcessError::Recoverable);
                    },
                };
        
                Ok(IsOutStage::AL(ToAlStage { msg, srca, srcb, al_ctrl, wb_ctrl, gpr_waddr, csr_waddr: (imm & 0xFFF) as u16, }))
            }

            InstType::LS => {
                let ls_ctrl = stage.ls_ctrl;

                Ok(IsOutStage::LS(ToLsStage {
                    msg, 
                    ls_ctrl,
                    gpr_waddr,

                    addr: rs1_val.wrapping_add(imm),
                    data: rs2_val,
                }))
            }
        }
    }
}
