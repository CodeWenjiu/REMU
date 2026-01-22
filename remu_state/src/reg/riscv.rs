use crate::reg::{RegCmd, RegOption};

pub struct RiscvReg {
    pub pc: u32,
    gpr: [u32; 32],
    tracer: remu_types::TracerDyn,
}

impl RiscvReg {
    pub fn new(opt: RegOption, tracer: remu_types::TracerDyn) -> Self {
        Self {
            pc: opt.init_pc,
            gpr: [0; 32],
            tracer,
        }
    }

    #[inline(always)]
    pub fn read_gpr(&self, index: usize) -> u32 {
        unsafe { *self.gpr.get_unchecked(index) }
    }

    #[inline(always)]
    pub fn write_gpr(&mut self, index: usize, value: u32) {
        unsafe { *self.gpr.get_unchecked_mut(index) = value }
    }

    pub(crate) fn execute(&mut self, cmd: &RegCmd) {
        match cmd {
            RegCmd::Read { index } => {
                self.tracer
                    .borrow()
                    .reg_show(*index, self.read_gpr(index.idx()));
            }
            RegCmd::Print { range } => {
                let regs: [(remu_types::Gpr, u32); 32] = core::array::from_fn(|i| {
                    let reg =
                        remu_types::Gpr::from_repr(i).expect("valid RISC-V GPR index (0..=31)");
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
