//! Vector context and SEW abstraction. Parsed once per instruction, reused across element loops.

use remu_types::isa::{
    extension_v::VExtensionConfig,
    reg::VectorCsrState,
    RvIsa,
};

/// VLMAX in elements (standard formula, valid for fractional LMUL).
pub(crate) fn calculate_vlmax(vlenb: u32, vtype: u32) -> u32 {
    let vsew = (vtype >> 3) & 0x7;
    let vlmul = vtype & 0x7;
    let lmul_shift: i8 = match vlmul {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        5 => -3,
        6 => -2,
        7 => -1,
        _ => return 0,
    };
    let sew_shift: i8 = match vsew {
        0..=3 => -(vsew as i8),
        _ => return 0,
    };
    let total_shift = lmul_shift + sew_shift;
    if total_shift >= 0 {
        vlenb << total_shift
    } else {
        vlenb >> (-total_shift)
    }
}

/// Number of register groups nf (1/2/4/8) from vlmul.
pub(crate) fn nf_from_vlmul(vlmul: u32) -> usize {
    match vlmul & 0x7 {
        0 => 1,
        1 => 2,
        2 => 4,
        3 => 8,
        _ => 1,
    }
}

/// SEW (element width) for vector ops. Eliminates repeated match sew_bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(usize)]
pub(crate) enum Sew {
    E8 = 1,
    E16 = 2,
    E32 = 4,
    E64 = 8,
}

impl Sew {
    #[inline]
    pub(crate) fn from_vtype(vtype: u32) -> Self {
        let vsew = (vtype >> 3) & 0x7;
        match vsew {
            0 => Sew::E8,
            1 => Sew::E16,
            2 => Sew::E32,
            3 => Sew::E64,
            _ => Sew::E32,
        }
    }

    #[inline]
    pub(crate) fn bytes(self) -> usize {
        self as usize
    }

    /// Read element as u64 from chunk at offset.
    #[inline]
    pub(crate) fn read_u(self, chunk: &[u8], off: usize) -> u64 {
        match self {
            Sew::E8 => chunk.get(off).copied().unwrap_or(0) as u64,
            Sew::E16 => u16::from_le_bytes(chunk[off..off + 2].try_into().unwrap_or([0, 0])) as u64,
            Sew::E32 => u32::from_le_bytes(chunk[off..off + 4].try_into().unwrap_or([0; 4])) as u64,
            Sew::E64 => u64::from_le_bytes(chunk[off..off + 8].try_into().unwrap_or([0; 8])),
        }
    }

    /// Read element as i64 (sign-extended).
    #[inline]
    pub(crate) fn read_i(self, chunk: &[u8], off: usize) -> i64 {
        match self {
            Sew::E8 => chunk.get(off).copied().unwrap_or(0) as i8 as i64,
            Sew::E16 => i16::from_le_bytes(chunk[off..off + 2].try_into().unwrap_or([0, 0])) as i64,
            Sew::E32 => i32::from_le_bytes(chunk[off..off + 4].try_into().unwrap_or([0; 4])) as i64,
            Sew::E64 => i64::from_le_bytes(chunk[off..off + 8].try_into().unwrap_or([0; 8])),
        }
    }

    /// Write u64 to chunk at offset (truncates to SEW).
    #[inline]
    pub(crate) fn write(self, chunk: &mut [u8], off: usize, val: u64) {
        match self {
            Sew::E8 => {
                if let Some(b) = chunk.get_mut(off) {
                    *b = val as u8;
                }
            }
            Sew::E16 => chunk[off..off + 2].copy_from_slice(&(val as u16).to_le_bytes()),
            Sew::E32 => chunk[off..off + 4].copy_from_slice(&(val as u32).to_le_bytes()),
            Sew::E64 => chunk[off..off + 8].copy_from_slice(&val.to_le_bytes()),
        }
    }
}

/// Parsed vtype/vl context. Build once, use for all element loops in an instruction.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VContext {
    pub vl: u32,
    #[allow(dead_code)]
    pub vtype: u32,
    pub vlmul: u32,
    #[allow(dead_code)]
    pub vsew: u32,
    pub sew: Sew,
    pub sew_bytes: usize,
    pub vlenb: usize,
    pub nf: usize,
    pub vlmax: u32,
}

impl VContext {
    pub(crate) fn from_state<P, C>(ctx: &mut C) -> Self
    where
        P: remu_state::StatePolicy,
        C: crate::ExecuteContext<P>,
    {
        let state = ctx.state_mut();
        let vl = state.reg.csr.vector.vl();
        let vtype = state.reg.csr.vector.vtype();
        let vlmul = vtype & 0x7;
        let vsew = (vtype >> 3) & 0x7;
        let sew = Sew::from_vtype(vtype);
        let sew_bytes = sew.bytes();
        let vlenb =
            <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
        let nf = nf_from_vlmul(vlmul);
        let vlmax = calculate_vlmax(vlenb as u32, vtype);
        Self {
            vl,
            vtype,
            vlmul,
            vsew,
            sew,
            sew_bytes,
            vlenb,
            nf,
            vlmax,
        }
    }

    /// Effective element count for this instruction.
    #[inline]
    pub(crate) fn n_elems(self) -> u32 {
        let total = (self.nf * self.vlenb) / self.sew_bytes;
        self.vl.min(total as u32)
    }

    /// Byte offset and register index for element i.
    #[inline]
    pub(crate) fn elem_layout(self, elem_idx: usize) -> (usize, usize, usize) {
        let byte_off = elem_idx * self.sew_bytes;
        let reg_i = byte_off / self.vlenb;
        let off = byte_off % self.vlenb;
        (byte_off, reg_i, off)
    }
}

/// Check mask bit at element index i.
#[inline]
pub(crate) fn mask_bit(v0: &[u8], i: usize) -> bool {
    (v0.get(i / 8).copied().unwrap_or(0) >> (i % 8)) & 1 != 0
}

/// Register range and overlap checks for vector ops.
pub(crate) mod vreg_check {
    use remu_state::{bus::BusError, StateError};

    #[inline]
    pub(crate) fn in_range(reg: usize, nf: usize) -> bool {
        reg + nf <= 32
    }

    #[inline]
    pub(crate) fn no_overlap(a: usize, na: usize, b: usize, nb: usize) -> bool {
        a + na <= b || b + nb <= a
    }

    pub(crate) fn check_regs(
        rd: usize,
        nf_rd: usize,
        rs1: Option<(usize, usize)>,
        rs2: Option<(usize, usize)>,
        v0_check: bool,
    ) -> Result<(), StateError> {
        if !in_range(rd, nf_rd) {
            return Err(StateError::BusError(Box::new(BusError::unmapped(0))));
        }
        if let Some((r, n)) = rs1 {
            if !in_range(r, n) {
                return Err(StateError::BusError(Box::new(BusError::unmapped(0))));
            }
            if !no_overlap(rd, nf_rd, r, n) {
                return Err(StateError::BusError(Box::new(BusError::unmapped(0))));
            }
        }
        if let Some((r, n)) = rs2 {
            if !in_range(r, n) {
                return Err(StateError::BusError(Box::new(BusError::unmapped(0))));
            }
            if !no_overlap(rd, nf_rd, r, n) {
                return Err(StateError::BusError(Box::new(BusError::unmapped(0))));
            }
        }
        if v0_check && !no_overlap(rd, nf_rd, 0, 1) {
            return Err(StateError::BusError(Box::new(BusError::unmapped(0))));
        }
        Ok(())
    }
}
