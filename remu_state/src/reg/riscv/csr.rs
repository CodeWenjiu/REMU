use remu_types::isa::extension_v::CsrConfig;
use remu_types::isa::reg::{Csr as CsrKind, VectorCsrState};

#[derive(Clone)]
pub struct Csr<C: CsrConfig> {
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

    // Vector CSRs: from config (same as FprState: () vs FprRegs).
    pub vector: C::VectorCsrState,
}

impl<C: CsrConfig> Default for Csr<C> {
    fn default() -> Self {
        Self {
            mstatus: 0x0000_1800,
            mie: 0,
            mtvec: 0,
            mscratch: 0,
            mepc: 0,
            mcause: 0,
            mtval: 0,
            mip: 0,
            vector: C::VectorCsrState::default(),
        }
    }
}

impl<C: CsrConfig> std::fmt::Debug for Csr<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Csr")
            .field("mstatus", &self.mstatus)
            .field("mie", &self.mie)
            .field("mtvec", &self.mtvec)
            .field("mscratch", &self.mscratch)
            .field("mepc", &self.mepc)
            .field("mcause", &self.mcause)
            .field("mtval", &self.mtval)
            .field("mip", &self.mip)
            .field("vector", &self.vector)
            .finish()
    }
}

impl<C: CsrConfig> Csr<C> {
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
            CsrKind::Vstart => self.vector.vstart(),
            CsrKind::Vxsat => self.vector.vxsat() & 1,
            CsrKind::Vxrm => self.vector.vxrm() & 3,
            CsrKind::Vcsr => self.vector.vcsr() & 7,
            CsrKind::Vl => self.vector.vl(),
            CsrKind::Vtype => self.vector.vtype(),
            CsrKind::Vlenb => <C::VectorCsrState as VectorCsrState>::VLENB,
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
            CsrKind::Vstart => self.vector.set_vstart(value),
            CsrKind::Vxsat => self.vector.set_vxsat(value & 1),
            CsrKind::Vxrm => self.vector.set_vxrm(value & 3),
            CsrKind::Vcsr => {
                self.vector.set_vcsr(value & 7);
                self.vector.set_vxsat(value & 1);
                self.vector.set_vxrm((value >> 1) & 3);
            }
            CsrKind::Vl => self.vector.set_vl(value),
            CsrKind::Vtype => self.vector.set_vtype(value),
            CsrKind::Vlenb => {} // read-only
            _ => {} // Misa and other read-only: no-op
        }
    }
}
