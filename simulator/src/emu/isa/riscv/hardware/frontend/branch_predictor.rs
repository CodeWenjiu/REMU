use state::cache::{BtbData, CacheTrait};

use crate::emu::{isa::riscv::hardware::frontend::ToIfStage, EmuHardware};

impl EmuHardware {
    fn branch_predict(&mut self) -> u32 {
        let pc = self.pipeline.pipeline_pc;

        let snpc = pc.wrapping_add(4);

        let npc = if let Some(target) = self.states.cache.btb.as_mut().unwrap().read(pc){
            target.target
        } else {
            snpc
        };

        npc
    }

    pub fn self_pipeline_branch_predict(&mut self) -> ToIfStage {
        let pc = self.pipeline.pipeline_pc;
        let npc = self.branch_predict();
        ToIfStage::new(pc, npc)
    }

    pub fn self_pipeline_branch_predict_update(&mut self) {
        let npc = self.branch_predict();
        self.pipeline.pipeline_pc = npc;
    }

    pub fn self_pipeline_branch_predict_flush(&mut self, pc: u32, target: u32) {
        self.states.cache.btb.as_mut().unwrap().replace(pc, BtbData {target});

        self.pipeline.pipeline_pc = target;
    }
}
