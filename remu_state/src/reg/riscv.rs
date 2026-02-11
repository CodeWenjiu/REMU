use remu_types::isa::reg::{Csr as CsrKind, Gpr};
use remu_types::isa::{RvIsa, reg::RegAccess};

use crate::reg::{CsrRegCmd, FprRegCmd, GprRegCmd, PcRegCmd, RegCmd, RegOption};

#[derive(Debug, Clone)]
pub struct Csr {
    // Machine Trap Setup
    pub mstatus: u32,
    pub mie: u32,
    pub mtvec: u32,
    // Machine Trap Handling
    pub mscratch: u32,
    pub mepc: u32,
    pub mcause: u32,
    pub mtval: u32,
    pub mip: u32,
}

impl Default for Csr {
    fn default() -> Self {
        Self {
            mstatus: 0x0000_1800, // MPP = 3 (machine mode)
            mie: 0,
            mtvec: 0,
            mscratch: 0,
            mepc: 0,
            mcause: 0,
            mtval: 0,
            mip: 0,
        }
    }
}

impl Csr {
    // --- mstatus 位 (RISC-V Privileged) ---
    const MSTATUS_MIE: u32 = 1 << 3;
    const MSTATUS_MPIE: u32 = 1 << 7;
    const MSTATUS_MPP_MASK: u32 = 3 << 11;
    const MSTATUS_MPP_MACHINE: u32 = 3 << 11;

    #[inline(always)]
    pub fn mstatus_mie(&self) -> bool {
        (self.mstatus & Self::MSTATUS_MIE) != 0
    }

    #[inline(always)]
    pub fn set_mstatus_mie(&mut self, v: bool) {
        if v {
            self.mstatus |= Self::MSTATUS_MIE;
        } else {
            self.mstatus &= !Self::MSTATUS_MIE;
        }
    }

    #[inline(always)]
    pub fn mstatus_mpie(&self) -> bool {
        (self.mstatus & Self::MSTATUS_MPIE) != 0
    }

    #[inline(always)]
    pub fn set_mstatus_mpie(&mut self, v: bool) {
        if v {
            self.mstatus |= Self::MSTATUS_MPIE;
        } else {
            self.mstatus &= !Self::MSTATUS_MPIE;
        }
    }

    #[inline(always)]
    pub fn mstatus_mpp(&self) -> u32 {
        (self.mstatus & Self::MSTATUS_MPP_MASK) >> 11
    }

    #[inline(always)]
    pub fn set_mstatus_mpp(&mut self, v: u32) {
        self.mstatus = (self.mstatus & !Self::MSTATUS_MPP_MASK) | ((v & 3) << 11);
    }

    #[inline(always)]
    pub fn mstatus_apply_trap_entry(&mut self) {
        let mie = self.mstatus_mie();
        self.set_mstatus_mie(false);
        self.set_mstatus_mpie(mie);
        self.set_mstatus_mpp(Self::MSTATUS_MPP_MACHINE >> 11);
    }

    #[inline(always)]
    pub fn mtvec_base(&self) -> u32 {
        self.mtvec & !3u32
    }

    pub fn read(&self, reg: CsrKind) -> u32 {
        match reg {
            CsrKind::Mstatus => self.mstatus,
            CsrKind::Mie => self.mie,
            CsrKind::Mtvec => self.mtvec,
            CsrKind::Mscratch => self.mscratch,
            CsrKind::Mepc => self.mepc,
            CsrKind::Mcause => self.mcause,
            CsrKind::Mtval => self.mtval,
            CsrKind::Mip => self.mip,
            _ => 0,
        }
    }

    pub fn write(&mut self, reg: CsrKind, value: u32) {
        match reg {
            CsrKind::Mstatus => self.mstatus = value,
            CsrKind::Mie => self.mie = value,
            CsrKind::Mtvec => self.mtvec = value,
            CsrKind::Mscratch => self.mscratch = value,
            CsrKind::Mepc => self.mepc = value,
            CsrKind::Mcause => self.mcause = value,
            CsrKind::Mtval => self.mtval = value,
            CsrKind::Mip => self.mip = value,
            _ => (), // Misa and other read-only: no-op
        }
    }
}

pub struct RiscvReg<I: RvIsa> {
    pub pc: I::PcState,
    pub gpr: I::GprState,
    pub fpr: I::FprState,
    pub csr: Csr,
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
