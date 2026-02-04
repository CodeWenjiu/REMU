remu_macro::mod_flat!(gpr, fpr);

use core::ops::{Deref, DerefMut, Index};

use crate::AllUsize;

pub trait RegDiff {
    fn diff(ref_this: &Self, dut: &Self) -> Vec<(String, AllUsize, AllUsize)>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PcState(pub u32);

impl Deref for PcState {
    type Target = u32;
    #[inline(always)]
    fn deref(&self) -> &u32 {
        &self.0
    }
}
impl DerefMut for PcState {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut u32 {
        &mut self.0
    }
}
impl From<u32> for PcState {
    #[inline(always)]
    fn from(x: u32) -> Self {
        PcState(x)
    }
}
impl RegDiff for PcState {
    fn diff(ref_this: &PcState, dut: &PcState) -> Vec<(String, AllUsize, AllUsize)> {
        if ref_this.0 != dut.0 {
            vec![(
                "pc".to_string(),
                AllUsize::U32(ref_this.0),
                AllUsize::U32(dut.0),
            )]
        } else {
            vec![]
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GprState(pub [u32; 32]);

impl Default for GprState {
    fn default() -> Self {
        GprState([0; 32])
    }
}
impl RegAccess for GprState {
    type Item = u32;
    #[inline(always)]
    fn raw_read(&self, idx: usize) -> u32 {
        self.0.raw_read(idx)
    }
    #[inline(always)]
    fn raw_write(&mut self, idx: usize, val: u32) {
        if idx != 0 {
            self.0.raw_write(idx, val);
        }
    }
}
impl Index<usize> for GprState {
    type Output = u32;
    #[inline(always)]
    fn index(&self, i: usize) -> &u32 {
        &self.0[i]
    }
}
impl RegDiff for GprState {
    fn diff(ref_this: &GprState, dut: &GprState) -> Vec<(String, AllUsize, AllUsize)> {
        (0..32)
            .filter_map(|i| {
                let (r, d) = (ref_this.0.raw_read(i), dut.0.raw_read(i));
                if r != d {
                    let name = Gpr::from_repr(i)
                        .map(|g| g.to_string())
                        .unwrap_or_else(|| format!("x{i}"));
                    Some((name, AllUsize::U32(r), AllUsize::U32(d)))
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

impl RegAccess for [u32; 32] {
    type Item = u32;

    #[inline(always)]
    fn raw_read(&self, idx: usize) -> Self::Item {
        unsafe { *self.get_unchecked(idx) }
    }

    #[inline(always)]
    fn raw_write(&mut self, idx: usize, val: Self::Item) {
        unsafe { *self.get_unchecked_mut(idx) = val }
    }
}

impl RegAccess for [u64; 32] {
    type Item = u64;

    #[inline(always)]
    fn raw_read(&self, idx: usize) -> Self::Item {
        unsafe { *self.get_unchecked(idx) }
    }

    #[inline(always)]
    fn raw_write(&mut self, idx: usize, val: Self::Item) {
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
