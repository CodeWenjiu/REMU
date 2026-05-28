//! Vector extension CSR state: presence and VLENB, dispatched by type (same pattern as FprState).
//! Only declares the storage interface; actual CSR read/write logic lives in remu_state/reg/riscv.

/// State for vector CSRs. When the V extension is absent, use `()` (zero size).
/// When present, use [`VectorCsrFields`] with the appropriate VLENB.
/// Read/write semantics (masks, vcsr composition) are implemented in state layer.
pub trait VectorCsrState: Default + Clone + std::fmt::Debug {
    /// VLEN/8 in bytes; used as the return value of the vlenb CSR.
    const VLENB: u32;

    fn vstart(&self) -> u32;
    fn vxsat(&self) -> u32;
    fn vxrm(&self) -> u32;
    fn vcsr(&self) -> u32;
    fn vl(&self) -> u32;
    fn vtype(&self) -> u32;

    fn set_vstart(&mut self, v: u32);
    fn set_vxsat(&mut self, v: u32);
    fn set_vxrm(&mut self, v: u32);
    fn set_vcsr(&mut self, v: u32);
    fn set_vl(&mut self, v: u32);
    fn set_vtype(&mut self, v: u32);
}

impl VectorCsrState for () {
    const VLENB: u32 = 0;

    #[inline(always)]
    fn vstart(&self) -> u32 {
        0
    }
    #[inline(always)]
    fn vxsat(&self) -> u32 {
        0
    }
    #[inline(always)]
    fn vxrm(&self) -> u32 {
        0
    }
    #[inline(always)]
    fn vcsr(&self) -> u32 {
        0
    }
    #[inline(always)]
    fn vl(&self) -> u32 {
        0
    }
    #[inline(always)]
    fn vtype(&self) -> u32 {
        0
    }

    #[inline(always)]
    fn set_vstart(&mut self, _: u32) {}
    #[inline(always)]
    fn set_vxsat(&mut self, _: u32) {}
    #[inline(always)]
    fn set_vxrm(&mut self, _: u32) {}
    #[inline(always)]
    fn set_vcsr(&mut self, _: u32) {}
    #[inline(always)]
    fn set_vl(&mut self, _: u32) {}
    #[inline(always)]
    fn set_vtype(&mut self, _: u32) {}
}

/// Vector CSR storage when the V extension is present. VLENB is the const generic.
#[derive(Clone, Copy)]
pub struct VectorCsrFields<const VLENB: u32> {
    pub vstart: u32,
    pub vxsat: u32,
    pub vxrm: u32,
    pub vcsr: u32,
    pub vl: u32,
    pub vtype: u32,
}

impl<const VLENB: u32> Default for VectorCsrFields<VLENB> {
    fn default() -> Self {
        Self {
            vstart: 0,
            vxsat: 0,
            vxrm: 0,
            vcsr: 0,
            vl: 0,
            vtype: 0x8000_0000, // vill=1 until vsetvli
        }
    }
}

impl<const VLENB: u32> std::fmt::Debug for VectorCsrFields<VLENB> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VectorCsrFields")
            .field("vstart", &self.vstart)
            .field("vxsat", &self.vxsat)
            .field("vxrm", &self.vxrm)
            .field("vcsr", &self.vcsr)
            .field("vl", &self.vl)
            .field("vtype", &self.vtype)
            .finish()
    }
}

impl<const VLENB: u32> VectorCsrState for VectorCsrFields<VLENB> {
    const VLENB: u32 = VLENB;

    #[inline(always)]
    fn vstart(&self) -> u32 {
        self.vstart
    }
    #[inline(always)]
    fn vxsat(&self) -> u32 {
        self.vxsat
    }
    #[inline(always)]
    fn vxrm(&self) -> u32 {
        self.vxrm
    }
    #[inline(always)]
    fn vcsr(&self) -> u32 {
        self.vcsr
    }
    #[inline(always)]
    fn vl(&self) -> u32 {
        self.vl
    }
    #[inline(always)]
    fn vtype(&self) -> u32 {
        self.vtype
    }

    #[inline(always)]
    fn set_vstart(&mut self, v: u32) {
        self.vstart = v;
    }
    #[inline(always)]
    fn set_vxsat(&mut self, v: u32) {
        self.vxsat = v;
    }
    #[inline(always)]
    fn set_vxrm(&mut self, v: u32) {
        self.vxrm = v;
    }
    #[inline(always)]
    fn set_vcsr(&mut self, v: u32) {
        self.vcsr = v;
    }
    #[inline(always)]
    fn set_vl(&mut self, v: u32) {
        self.vl = v;
    }
    #[inline(always)]
    fn set_vtype(&mut self, v: u32) {
        self.vtype = v;
    }
}
