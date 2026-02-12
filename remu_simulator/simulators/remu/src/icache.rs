use crate::riscv::inst::DecodedInst;

/// I-cache entry: fetch address and decoded instruction.
#[derive(Clone, Copy)]
pub struct CacheEntry {
    pub(crate) addr: u32,
    pub(crate) decoded: DecodedInst,
}

/// Instruction cache. `SIZE` must be a power of 2 so that index `(pc as usize) & (SIZE - 1)` is in bounds.
pub struct Icache<const SIZE: usize> {
    data: Box<[Option<CacheEntry>; SIZE]>,
}

impl<const SIZE: usize> Icache<SIZE> {
    /// Creates an empty I-cache. Panics if `SIZE` is not a power of 2.
    pub fn new() -> Self {
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

    /// Returns the slot for `pc`. Index is in `0..SIZE` when SIZE is a power of 2.
    #[inline(always)]
    pub fn slot_mut(&mut self, pc: u32) -> &mut Option<CacheEntry> {
        let i = Self::index(pc);
        unsafe { self.data.get_unchecked_mut(i) }
    }
}

impl<const SIZE: usize> Default for Icache<SIZE> {
    fn default() -> Self {
        Self::new()
    }
}
