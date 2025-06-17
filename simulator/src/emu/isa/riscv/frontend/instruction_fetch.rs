use remu_macro::log_err;
use remu_utils::{ProcessError, ProcessResult};

use crate::emu::Emu;

use super::ToIdStage;

#[derive(Default)]
pub struct ToIfStage {
    pub pc: u32,
}

impl Emu {
    pub fn instruction_fetch_rv32i(&mut self, stage: ToIfStage) -> ProcessResult<ToIdStage> {
        let pc = stage.pc;
        let inst = log_err!(
            self.states.mmu.read(pc, state::mmu::Mask::Word),
            ProcessError::Recoverable
        )?.1;

        Ok(ToIdStage {
            pc,
            inst,
        })
    }
}
