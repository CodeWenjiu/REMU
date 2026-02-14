use remu_types::isa::reg::{Csr as CsrKind, Gpr};
use remu_types::isa::{RvIsa, reg::RegAccess};

use crate::reg::{CsrRegCmd, FprRegCmd, GprRegCmd, PcRegCmd, RegCmd, RegOption};

use super::Csr;

pub struct RiscvReg<I: RvIsa> {
    pub pc: I::PcState,
    pub gpr: I::GprState,
    pub fpr: I::FprState,
    pub csr: Csr<I::VectorCsrState>,
    tracer: remu_types::TracerDyn,
}

impl<I: RvIsa> RiscvReg<I> {
    pub(crate) fn new(opt: RegOption, tracer: remu_types::TracerDyn) -> Self {
        Self {
            pc: opt.init_pc.into(),
            gpr: Default::default(),
            fpr: Default::default(),
            csr: Csr::default(),
            tracer,
        }
    }

    /// Read CSR value: from state for stateful CSRs, from ISA for read-only (e.g. Misa).
    #[inline(always)]
    pub fn read_csr(&self, reg: CsrKind) -> u32 {
        if reg == CsrKind::Misa {
            I::MISA
        } else {
            self.csr.read(reg)
        }
    }

    pub(crate) fn execute(&mut self, cmd: &RegCmd) {
        match cmd {
            RegCmd::Pc { subcmd } => self.execute_pc(subcmd),
            RegCmd::Csr { subcmd } => self.execute_csr(subcmd),
            RegCmd::Gpr { subcmd } => self.execute_gpr(subcmd),
            RegCmd::Fpr { subcmd } => self.execute_fpr(subcmd),
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

    fn execute_csr(&mut self, cmd: &CsrRegCmd) {
        match cmd {
            CsrRegCmd::Read { index } => {
                let value = self.read_csr(*index);
                self.tracer
                    .borrow()
                    .print(&format!("{} = {:#010x}", index, value));
            }
            CsrRegCmd::Write { index, value } => {
                self.csr.write(*index, *value);
            }
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
                self.tracer.borrow().reg_show_fpr(i, self.fpr.raw_read(i));
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
}
