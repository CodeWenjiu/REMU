remu_macro::mod_prv!(gpr, fpr, vr);
remu_macro::mod_pub!(csr);
pub use csr::*;
pub use fpr::Fpr;
pub use gpr::Gpr;
pub use vr::VrState;

use core::ops::{Deref, DerefMut, Index};

use crate::AllUsize;
pub use crate::wordlen::{IntoAllUsize, Xlen};

pub trait RegDiff {
    fn diff(ref_this: &Self, dut: &Self) -> Vec<(String, AllUsize, AllUsize)>;
}

/// PC state; the word is the ISA's XLEN type so RV32 and RV64 get distinct
/// monomorphizations with no runtime dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PcState<W: Xlen>(pub W);

impl<W: Xlen> Deref for PcState<W> {
    type Target = W;
    #[inline(always)]
    fn deref(&self) -> &W {
        &self.0
    }
}
impl<W: Xlen> DerefMut for PcState<W> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut W {
        &mut self.0
    }
}
impl<W: Xlen> From<W> for PcState<W> {
    #[inline(always)]
    fn from(x: W) -> Self {
        PcState(x)
    }
}
impl<W: Xlen> PcState<W> {
    #[inline(always)]
    pub fn wrapping_add(self, rhs: W) -> Self {
        PcState(self.0.wrapping_add(rhs))
    }
    #[inline(always)]
    pub fn wrapping_sub(self, rhs: W) -> Self {
        PcState(self.0.wrapping_sub(rhs))
    }
}
impl<W: Xlen + IntoAllUsize> RegDiff for PcState<W> {
    fn diff(ref_this: &PcState<W>, dut: &PcState<W>) -> Vec<(String, AllUsize, AllUsize)> {
        if ref_this.0 != dut.0 {
            vec![("pc".to_string(), ref_this.0.into_all(), dut.0.into_all())]
        } else {
            vec![]
        }
    }
}

/// GPR state; the word is the ISA's XLEN type (see [`PcState`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GprState<W: Xlen>(pub [W; 32]);

impl<W: Xlen> Default for GprState<W> {
    fn default() -> Self {
        GprState([W::default(); 32])
    }
}
impl<W: Xlen> RegAccess for GprState<W> {
    type Item = W;
    #[inline(always)]
    fn raw_read(&self, idx: usize) -> W {
        self.0.raw_read(idx)
    }
    #[inline(always)]
    fn raw_write(&mut self, idx: usize, val: W) {
        if idx != 0 {
            self.0.raw_write(idx, val);
        }
    }
}
impl<W: Xlen> Index<usize> for GprState<W> {
    type Output = W;
    #[inline(always)]
    fn index(&self, i: usize) -> &W {
        &self.0[i]
    }
}
impl<W: Xlen + IntoAllUsize> RegDiff for GprState<W> {
    fn diff(ref_this: &GprState<W>, dut: &GprState<W>) -> Vec<(String, AllUsize, AllUsize)> {
        (0..32)
            .filter_map(|i| {
                let (r, d) = (ref_this.0.raw_read(i), dut.0.raw_read(i));
                if r != d {
                    let name = Gpr::from_repr(i)
                        .map(|g| g.to_string())
                        .unwrap_or_else(|| format!("x{i}"));
                    Some((name, r.into_all(), d.into_all()))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FprRegs(pub [u32; 32]);

impl Default for FprRegs {
    fn default() -> Self {
        FprRegs([0; 32])
    }
}
impl RegAccess for FprRegs {
    type Item = u32;
    #[inline(always)]
    fn raw_read(&self, idx: usize) -> u32 {
        self.0.raw_read(idx)
    }
    #[inline(always)]
    fn raw_write(&mut self, idx: usize, val: u32) {
        self.0.raw_write(idx, val);
    }
}
impl RegDiff for FprRegs {
    fn diff(ref_this: &FprRegs, dut: &FprRegs) -> Vec<(String, AllUsize, AllUsize)> {
        (0..32)
            .filter_map(|i| {
                let (r, d) = (ref_this.0.raw_read(i), dut.0.raw_read(i));
                if r != d {
                    let name = Fpr::from_repr(i)
                        .map(|f| f.to_string())
                        .unwrap_or_else(|| format!("f{i}"));
                    Some((name, AllUsize::U32(r), AllUsize::U32(d)))
                } else {
                    None
                }
            })
            .collect()
    }
}
impl RegDiff for () {
    fn diff(_: &(), _: &()) -> Vec<(String, AllUsize, AllUsize)> {
        vec![]
    }
}

pub trait RegAccess {
    type Item: Copy + std::fmt::Debug;

    fn raw_read(&self, idx: usize) -> Self::Item;
    fn raw_write(&mut self, idx: usize, val: Self::Item);
}

impl<W: Xlen> RegAccess for [W; 32] {
    type Item = W;

    #[inline(always)]
    fn raw_read(&self, idx: usize) -> W {
        unsafe { *self.get_unchecked(idx) }
    }

    #[inline(always)]
    fn raw_write(&mut self, idx: usize, val: W) {
        unsafe { *self.get_unchecked_mut(idx) = val }
    }
}

impl RegAccess for () {
    type Item = u32;

    #[inline(always)]
    fn raw_read(&self, _: usize) -> Self::Item {
        panic!("No FPU");
    }

    #[inline(always)]
    fn raw_write(&mut self, _: usize, _: Self::Item) {
        panic!("No FPU");
    }
}

pub trait FprAccess: RegAccess<Item = u32> {}
impl<T> FprAccess for T where T: RegAccess<Item = u32> {}
