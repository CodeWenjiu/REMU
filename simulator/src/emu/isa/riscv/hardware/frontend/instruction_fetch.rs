use remu_macro::log_err;
use remu_utils::{ProcessError, ProcessResult};

use crate::emu::{isa::riscv::BasicStageMsg, Emu};

use super::ToIdStage;

#[derive(Default)]
pub struct ToIfStage {
    pc: u32,
    npc: u32,
}

impl ToIfStage {
    pub fn new(pc: u32, npc: u32) -> Self {
        Self { pc, npc }
    }
}

impl Emu {
    pub fn instruction_fetch_rv32i(&mut self, stage: ToIfStage) -> ProcessResult<ToIdStage> {
        let msg = BasicStageMsg { pc: stage.pc, npc: stage.npc, trap: None };
        let inst = log_err!(
            self.states.mmu.read(msg.pc, state::mmu::Mask::Word),
            ProcessError::Recoverable
        )?.1;

        Ok(ToIdStage {
            msg,
            inst,
        })
    }
}
