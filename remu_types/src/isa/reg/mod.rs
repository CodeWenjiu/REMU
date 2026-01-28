remu_macro::mod_flat!(gpr);

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
