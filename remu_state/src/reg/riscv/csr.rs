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
            // MPP=M, VS=Off — matches Spike reset for difftest; Zve firmware must set VS (e.g. `pre_main_init`).
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
    // --- mstatus bits (RISC-V Privileged) ---
    const MSTATUS_MIE: u32 = 1 << 3;
    const MSTATUS_MPIE: u32 = 1 << 7;
    /// Vector extension state (VS): bits [10:9], same encoding as FS.
    const MSTATUS_VS_MASK: u32 = 0b11 << 9;
    const MSTATUS_FS_MASK: u32 = 0b11 << 13;
    const MSTATUS_XS_MASK: u32 = 0b11 << 15;
    /// Summary dirty (RV32): OR of FS/VS/XS dirty states.
    const MSTATUS_SD: u32 = 1 << 31;
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

    /// `mstatus.VS` field (0=Off, 1=Initial, 2=Clean, 3=Dirty).
    #[inline(always)]
    pub fn mstatus_vs(&self) -> u32 {
        (self.mstatus & Self::MSTATUS_VS_MASK) >> 9
    }

    /// VS == Off: vector architectural state must not be accessed.
    #[inline(always)]
    pub fn mstatus_vs_off(&self) -> bool {
        self.mstatus_vs() == 0
    }

    /// Mark vector extension state dirty after an instruction successfully updates vector arch state.
    #[inline(always)]
    pub fn set_mstatus_vs_dirty(&mut self) {
        self.mstatus = (self.mstatus & !Self::MSTATUS_VS_MASK) | (3 << 9);
        self.mstatus_refresh_sd();
    }

    /// Recompute read-only SD summary bit from FS / VS / XS.
    #[inline]
    pub fn mstatus_refresh_sd(&mut self) {
        let fs = (self.mstatus & Self::MSTATUS_FS_MASK) >> 13;
        let vs = (self.mstatus & Self::MSTATUS_VS_MASK) >> 9;
        let xs = (self.mstatus & Self::MSTATUS_XS_MASK) >> 15;
        let dirty = fs == 3 || vs == 3 || xs == 3;
        if dirty {
            self.mstatus |= Self::MSTATUS_SD;
        } else {
            self.mstatus &= !Self::MSTATUS_SD;
        }
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
            CsrKind::Mstatus => {
                self.mstatus = value;
                self.mstatus_refresh_sd();
            }
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
