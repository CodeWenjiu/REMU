use mmu::MMU;
use model::{JydPipeCell, PipelineModel};
use reg::AnyRegfile;
use remu_utils::ISA;

remu_macro::mod_pub!(mmu, reg, model);

#[derive(Clone)]
pub struct States {
    pub regfile: AnyRegfile,
    pub mmu: MMU,
    pub pipe_state: PipelineModel<JydPipeCell>,
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
        pipe_state: PipelineModel<JydPipeCell>,
    ) -> Result<Self, ()> {
        let regfile = reg::regfile_io_factory(isa, reset_vector)?;

        let mmu = MMU::new();

        Ok(States {
            regfile,
            mmu,
            pipe_state,
        })
    }
}
