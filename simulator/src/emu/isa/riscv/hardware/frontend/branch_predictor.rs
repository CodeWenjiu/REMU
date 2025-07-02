use state::cache::{BtbData, CacheTrait};

use crate::emu::{isa::riscv::hardware::frontend::ToIfStage, EmuHardware};

impl EmuHardware {
    fn branch_predict(&self) -> u32 {
        let pc = self.pipeline.pipeline_pc;
        if let Some(target) = self.states.cache.btb.as_ref().unwrap().borrow_mut().read(pc){
            target.target
        } else {
            pc.wrapping_add(4)
        }
    }

    pub fn self_pipeline_branch_predict(&self) -> ToIfStage {
        let pc = self.pipeline.pipeline_pc;
        let npc = self.branch_predict();
        ToIfStage::new(pc, npc)
    }

    pub fn self_pipeline_branch_predict_update(&mut self) {
        let npc = self.branch_predict();
        self.pipeline.pipeline_pc = npc;
    }

    pub fn self_pipeline_branch_predict_flush(&mut self, pc: u32, target: u32) {
        // self.get_btb_mut().update(pc, target);
        self.states.cache.btb.as_ref().unwrap().borrow_mut().replace(pc, BtbData {target});
        self.pipeline.pipeline_pc = target;
    }
}