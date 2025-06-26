use remu_utils::ProcessResult;
use state::reg::RegfileIo;

use crate::emu::EmuHardware;

use super::frontend::{IsOutStage, ToIfStage};

impl EmuHardware {
    pub fn self_step_cycle_singlecycle(&mut self) -> ProcessResult<()> {
        let pc = self.states.regfile.read_pc();

        let to_if = ToIfStage::new(pc, pc.wrapping_add(4));

        let to_id = self.instruction_fetch_rv32i(to_if)?;

        let inst = to_id.inst;
        
        let to_is = self.instruction_decode(to_id)?;
        let to_ex = self.instruction_issue(to_is)?;

        let to_wb = match to_ex {
            IsOutStage::AL(to_al) => {
                self.arithmetic_logic_rv32(to_al)?
            }

            IsOutStage::LS(to_ls) => {
                self.load_store_rv32i(to_ls)?
            }
        };

        let next_pc = self.write_back_rv32i(to_wb)?.next_pc;

        self.times.cycles += 1;
        self.times.instructions += 1;
        
        (self.callback.instruction_complete)(pc, next_pc, inst)?;

        Ok(())
    }
}