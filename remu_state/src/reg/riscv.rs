use remu_types::isa::{reg::RegAccess, RvIsa};
use remu_types::isa::reg::Gpr;

use crate::reg::{GprRegCmd, FprRegCmd, PcRegCmd, RegCmd, RegOption};

pub struct RiscvReg<I: RvIsa> {
    pub pc: I::PcState,
    pub gpr: I::GprState,
    pub fpr: I::FprState,
    tracer: remu_types::TracerDyn,
}

impl<I: RvIsa> RiscvReg<I> {
    pub(crate) fn new(opt: RegOption, tracer: remu_types::TracerDyn) -> Self {
        Self {
            pc: opt.init_pc.into(),
            gpr: Default::default(),
            fpr: Default::default(),
            tracer,
        }
    }

    pub(crate) fn execute(&mut self, cmd: &RegCmd) {
        match cmd {
            RegCmd::Gpr { subcmd } => self.execute_gpr(subcmd),
            RegCmd::Fpr { subcmd } => self.execute_fpr(subcmd),
            RegCmd::Pc { subcmd } => self.execute_pc(subcmd),
        }
    }

    fn execute_gpr(&mut self, cmd: &GprRegCmd) {
        match cmd {
            GprRegCmd::Read { index } => {
                self.tracer
                    .borrow()
                    .reg_show(*index, self.gpr.raw_read(index.idx()));
            }
            GprRegCmd::Print { range } => {
                let regs: [(Gpr, u32); 32] = core::array::from_fn(|i| {
                    let reg = Gpr::from_repr(i).expect("valid RISC-V GPR index (0..=31)");
                    (reg, self.gpr[i])
                });
                self.tracer.borrow().reg_print(&regs, range.clone());
            }
            GprRegCmd::Write { index, value } => {
                self.gpr.raw_write(index.idx(), *value);
            }
        }
    }

    fn execute_fpr(&mut self, cmd: &FprRegCmd) {
        match cmd {
            FprRegCmd::Read { index } => {
                let i = index.idx();
                self.tracer
                    .borrow()
                    .reg_show_fpr(i, self.fpr.raw_read(i));
            }
            FprRegCmd::Print { range } => {
                let regs: Vec<(usize, u32)> = (range.start..range.end)
                    .map(|i| (i, self.fpr.raw_read(i)))
                    .collect();
                self.tracer.borrow().reg_print_fpr(&regs, range.clone());
            }
            FprRegCmd::Write { index, value } => {
                self.fpr.raw_write(index.idx(), *value);
            }
        }
    }

    fn execute_pc(&mut self, cmd: &PcRegCmd) {
        match cmd {
            PcRegCmd::Read => {
                self.tracer.borrow().reg_show_pc(*self.pc);
            }
            PcRegCmd::Write { value } => {
                *self.pc = (*value).into();
            }
        }
    }
}
