use remu_types::isa::{
    ArchConfig, RvIsa,
    extension::Extension,
    reg::{Gpr, RegAccess},
};

use crate::reg::{RegCmd, RegOption};

pub struct RiscvReg<I: RvIsa> {
    pc: u32,
    gpr: [u32; 32],
    fpr: <<<I as RvIsa>::Conf as ArchConfig>::F as Extension>::State,
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

    #[inline(always)]
    pub fn read_pc(&self) -> u32 {
        self.pc
    }

    #[inline(always)]
    pub fn write_pc(&mut self, value: u32) {
        self.pc = value;
    }

    #[inline(always)]
    pub fn read_gpr(&self, index: usize) -> u32 {
        self.gpr.raw_read(index)
    }

    #[inline(always)]
    pub fn write_gpr(&mut self, index: usize, value: u32) {
        self.gpr.raw_write(index, value);
    }

    #[inline(always)]
    pub fn read_fpr(&self, index: usize) -> u32 {
        self.fpr.raw_read(index)
    }

    #[inline(always)]
    pub fn write_fpr(&mut self, index: usize, value: u32) {
        self.fpr.raw_write(index, value);
    }

    pub(crate) fn execute(&mut self, cmd: &RegCmd) {
        match cmd {
            RegCmd::Read { index } => {
                self.tracer
                    .borrow()
                    .reg_show(*index, self.read_gpr(index.idx()));
            }
            RegCmd::Print { range } => {
                let regs: [(Gpr, u32); 32] = core::array::from_fn(|i| {
                    let reg = Gpr::from_repr(i).expect("valid RISC-V GPR index (0..=31)");
                    (reg, self.gpr[i])
                });

                self.tracer.borrow().reg_print(&regs, range.clone());
            }
            RegCmd::Write { index, value } => {
                self.write_gpr(index.idx(), *value);
            }
        }
    }
}
