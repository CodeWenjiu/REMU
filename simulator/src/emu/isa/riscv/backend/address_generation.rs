use remu_utils::ProcessResult;

use crate::emu::Emu;

use super::{ToLsStage};
use super::super::{InstMsg, RV32ILS};

#[derive(Default)]
pub struct ToAgStage {
    pub pc: u32,
    pub inst: RV32ILS,
    pub msg: InstMsg, 
}

impl Emu {
    pub fn address_generation_rv32i(&self, stage: ToAgStage) -> ProcessResult<ToLsStage> {
        let pc = stage.pc;
        let inst = stage.inst;
        let imm = stage.msg.imm;
        let rs1_val = stage.msg.rs1;
        let rs2_val = stage.msg.rs2;
        let rd_addr = stage.msg.rd_addr;

        let addr = rs1_val.wrapping_add(imm);
        let data = rs2_val;

        Ok(ToLsStage{
            pc,
            inst,
            rd_addr,

            addr,
            data,
        })
    }
}
