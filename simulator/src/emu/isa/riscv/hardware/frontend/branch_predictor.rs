use state::cache::{BRMsg, BtbData, CacheBase};

use crate::emu::{isa::riscv::hardware::frontend::ToIfStage, EmuHardware};

impl EmuHardware {
    fn branch_predict(&mut self) -> u32 {
        let pc = self.pipeline.pipeline_pc;

        let snpc = pc.wrapping_add(4);

        let npc = if let Some(target_vec) = self.states.cache.btb.as_mut().unwrap().read(pc) {
            target_vec[0].target
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

    pub fn self_pipeline_branch_predict_flush(&mut self, pc: u32, target: u32, brmsg: BRMsg) {
        self.states.cache.btb.as_mut().unwrap().replace(pc, vec![BtbData {typ: brmsg.br_type, target}]);

        self.pipeline.pipeline_pc = target;
    }

    pub fn self_pipeline_branch_predict_bdb_update(&mut self, pc: u32, brmsg: BRMsg) {
        self.states.cache.btb.as_mut().unwrap().hyper_replace(pc, brmsg);
    }
}
