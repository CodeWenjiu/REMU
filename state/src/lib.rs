use mmu::MMU;
use model::StageModel;
use reg::AnyRegfile;
use remu_utils::ISA;
use crate::cache::Cache;

remu_macro::mod_pub!(mmu, reg, cache, model);

#[derive(Clone)]
pub struct States {
    pub regfile: AnyRegfile,
    pub mmu: MMU,
    pub pipe_state: Option<StageModel>,
    pub cache: Cache,
}

use bitflags::bitflags;


bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct CheckFlags4reg: u8 {
        const pc = 1 << 0;
        const gpr = 1 << 1;
        const csr = 1 << 2;
    }
}

pub struct CheckFlags {
    pub reg_flag: CheckFlags4reg,
}

impl States {
    pub fn new(
        isa: ISA,
        reset_vector: u32,
    ) -> Result<Self, ()> {
        let regfile = reg::regfile_io_factory(isa, reset_vector)?;

        let mmu = MMU::new();

        Ok(States {
            regfile,
            mmu,
            pipe_state: None,
            cache: Cache::new(),
        })
    }

    pub fn init_pipe(&mut self, pipe_state: Option<StageModel>) {
        self.pipe_state = pipe_state;
    }
}
