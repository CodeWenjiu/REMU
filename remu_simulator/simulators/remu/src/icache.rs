use crate::riscv::DecodedInst;

/// I-cache entry: fetch address and decoded instruction. `None` = empty slot.
/// `addr: u32` has a niche at `u32::MAX`, so `Option<CacheEntry>` is the same
/// size as `CacheEntry` (zero overhead) and the empty slot costs nothing.
#[derive(Clone, Copy)]
pub(crate) struct CacheEntry {
    pub(crate) addr: u32,
    pub(crate) decoded: DecodedInst,
}

/// Instruction cache. `SIZE` must be a power of 2 so that index `(pc as usize) & (SIZE - 1)` is in bounds.
pub(crate) struct Icache<const SIZE: usize> {
    data: Box<[Option<CacheEntry>; SIZE]>,
}

impl<const SIZE: usize> Icache<SIZE> {
    /// Creates an empty I-cache. Panics if `SIZE` is not a power of 2.
    pub(crate) fn new() -> Self {
        assert!(
            SIZE > 0 && (SIZE & (SIZE - 1)) == 0,
            "Icache SIZE must be a power of 2"
        );
        Self {
            data: Box::new([None; SIZE]),
        }
    }

    #[inline(always)]
    fn index(pc: u32) -> usize {
        (pc as usize) & (SIZE - 1)
    }

    /// Returns the entry slot for `pc`. Caller checks for hit (Some + addr == pc).
    /// `index()` masks with `SIZE - 1` (SIZE is a power of 2), so the index is always in bounds.
    #[inline(always)]
    pub(crate) fn get_entry_mut(&mut self, pc: u32) -> &mut Option<CacheEntry> {
        let i = Self::index(pc);
        unsafe { self.data.get_unchecked_mut(i) }
    }

    /// Invalidates the cache line for `pc`. Next fetch at this PC will refill from bus.
    #[inline(always)]
    pub(crate) fn invalidate(&mut self, pc: u32) {
        *self.get_entry_mut(pc) = None;
    }

    /// Clears all entries (e.g. after fence.i). Next fetch will refill.
    #[inline(never)]
    pub(crate) fn flush(&mut self) {
        for entry in self.data.iter_mut() {
            *entry = None;
        }
    }
}

impl<const SIZE: usize> Default for Icache<SIZE> {
    fn default() -> Self {
        Self::new()
    }
}
