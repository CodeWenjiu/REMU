use remu_types::isa::{
    ArchConfig, RvIsa,
    extension::Extension,
    reg::{Gpr, RegAccess},
};

use crate::reg::{RegCmd, RegOption};

pub struct RiscvReg<I: RvIsa> {
    pub pc: u32,
    pub gpr: [u32; 32],
    pub fpr: <<<I as RvIsa>::Conf as ArchConfig>::F as Extension>::State,
    tracer: remu_types::TracerDyn,
}

impl<I: RvIsa> RiscvReg<I> {
    pub(crate) fn new(opt: RegOption, tracer: remu_types::TracerDyn) -> Self {
        Self {
            pc: opt.init_pc,
            gpr: [0; 32],
            fpr: Default::default(),
            tracer,
        }
    }

    pub(crate) fn execute(&mut self, cmd: &RegCmd) {
        match cmd {
            RegCmd::Read { index } => {
                self.tracer
                    .borrow()
                    .reg_show(*index, self.gpr.raw_read(index.idx()));
            }
            RegCmd::Print { range } => {
                let regs: [(Gpr, u32); 32] = core::array::from_fn(|i| {
                    let reg = Gpr::from_repr(i).expect("valid RISC-V GPR index (0..=31)");
                    (reg, self.gpr[i])
                });

                self.tracer.borrow().reg_print(&regs, range.clone());
            }
            RegCmd::Write { index, value } => {
                self.gpr.raw_write(index.idx(), *value);
            }
        }
    }
}
