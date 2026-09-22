use remu_isa::isa::extension_v::CsrConfig;
use remu_isa::isa::reg::{Csr as CsrKind, PrivMode, VectorCsrState};

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

    // Supervisor Trap Setup / Handling (ch2: U/S-mode switching).
    // `sstatus` is a *view* of `mstatus` (its low bits); we store the same
    // bits separately so S-mode CSR reads/writes work without touching the
    // full mstatus value (remu has no virtualization, so no shadowing needed).
    pub sstatus: u32,
    pub sie: u32,
    pub stvec: u32,
    pub sscratch: u32,
    pub sepc: u32,
    pub scause: u32,
    pub stval: u32,
    pub sip: u32,

    /// Current privilege mode (not a CSR; tracked alongside the trap CSRs).
    pub priv_mode: PrivMode,

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
            sstatus: 0,
            sie: 0,
            stvec: 0,
            sscratch: 0,
            sepc: 0,
            scause: 0,
            stval: 0,
            sip: 0,
            priv_mode: PrivMode::Machine,
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
            .field("sstatus", &self.sstatus)
            .field("sie", &self.sie)
            .field("stvec", &self.stvec)
            .field("sscratch", &self.sscratch)
            .field("sepc", &self.sepc)
            .field("scause", &self.scause)
            .field("stval", &self.stval)
            .field("sip", &self.sip)
            .field("priv_mode", &self.priv_mode)
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

    /// Current privilege mode.
    #[inline(always)]
    pub fn priv_mode(&self) -> PrivMode {
        self.priv_mode
    }

    #[inline(always)]
    pub fn mstatus_apply_trap_entry(&mut self) {
        let mie = self.mstatus_mie();
        self.set_mstatus_mie(false);
        self.set_mstatus_mpie(mie);
        self.set_mstatus_mpp(self.priv_mode.bits());
        self.priv_mode = PrivMode::Machine;
    }

    /// `mret`: privilege returns to `mstatus.MPP`, interrupts restore from
    /// `MPIE`; `MPIE` is set and `MPP` is reset to M (U mode is not modeled).
    #[inline(always)]
    pub fn mstatus_apply_mret(&mut self) {
        let prv = PrivMode::from_bits(self.mstatus_mpp());
        let mpie = self.mstatus_mpie();
        self.set_mstatus_mie(mpie);
        self.set_mstatus_mpie(true);
        self.set_mstatus_mpp(PrivMode::Machine.bits());
        self.priv_mode = prv;
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

    #[inline(always)]
    pub fn stvec_base(&self) -> u32 {
        self.stvec & !3u32
    }

    /// `sstatus` is the S-mode view of `mstatus`: bits SIE(1), SPIE(5),
    /// SPP(8). Reading/writing keeps the S-mode CSR in sync with mstatus.
    /// (remu does not model UXL/SUM/MXR etc.; masks below match Spike's
    /// `MSTATUS_SSTATUS_MASK` low word.)
    const SSTATUS_MASK: u32 = 0x8000_0000 | 0x0000_0122; // SD | SPP | SPIE | SIE

    #[inline(always)]
    pub fn sstatus_read(&self) -> u32 {
        self.mstatus & Self::SSTATUS_MASK
    }

    #[inline(always)]
    pub fn sstatus_write(&mut self, value: u32) {
        let mask = Self::SSTATUS_MASK & !0x8000_0000; // SD is read-only
        self.mstatus = (self.mstatus & !mask) | (value & mask);
    }

    /// S-mode SPP = 1 means *trap from S*, 0 = from U.
    #[inline(always)]
    pub fn sstatus_spp(&self) -> bool {
        (self.mstatus & (1 << 8)) != 0
    }

    #[inline(always)]
    pub fn set_sstatus_spp(&mut self, v: bool) {
        if v {
            self.mstatus |= 1 << 8;
        } else {
            self.mstatus &= !(1 << 8);
        }
    }

    /// `sret`: return to `sstatus.SPP`, restart interrupts from `SPIE`, set
    /// `SPIE`, and clear `SPP` (back to U). The actual privilege switch is
    /// applied by the caller via [`priv_mode`](Self::priv_mode).
    #[inline(always)]
    pub fn sstatus_apply_sret(&mut self) {
        let spp = self.sstatus_spp();
        let spie = (self.mstatus >> 5) & 1 == 1; // SPIE
        // SIE <- SPIE
        if spie {
            self.mstatus |= 1 << 1;
        } else {
            self.mstatus &= !(1 << 1);
        }
        // SPIE <- 1, SPP <- 0 (U)
        self.mstatus |= 1 << 5;
        self.mstatus &= !(1 << 8);
        self.priv_mode = if spp {
            PrivMode::Supervisor
        } else {
            PrivMode::User
        };
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
            CsrKind::Sstatus => self.sstatus_read(),
            CsrKind::Sie => self.mie & 0x222, // S-mode view of mie (SSIE/STIE/SEIE)
            CsrKind::Stvec => self.stvec,
            CsrKind::Sscratch => self.sscratch,
            CsrKind::Sepc => self.sepc,
            CsrKind::Scause => self.scause,
            CsrKind::Stval => self.stval,
            CsrKind::Sip => self.mip & 0x222, // S-mode view of mip
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
            CsrKind::Sstatus => self.sstatus_write(value),
            CsrKind::Sie => self.mie = (self.mie & !0x222) | (value & 0x222),
            CsrKind::Stvec => self.stvec = value,
            CsrKind::Sscratch => self.sscratch = value,
            CsrKind::Sepc => self.sepc = value,
            CsrKind::Scause => self.scause = value,
            CsrKind::Stval => self.stval = value,
            CsrKind::Sip => self.mip = (self.mip & !0x222) | (value & 0x222),
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
            _ => {}              // Misa and other read-only: no-op
        }
    }
}
