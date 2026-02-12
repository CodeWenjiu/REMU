use crate::riscv::inst::DecodedInst;

/// Sentinel for empty slot. No valid fetch PC equals this (e.g. top of 32-bit space).
pub(crate) const INVALID_ADDR: u32 = u32::MAX;

/// I-cache entry: fetch address and decoded instruction. Empty slot = addr == INVALID_ADDR.
#[derive(Clone, Copy)]
pub struct CacheEntry {
    pub(crate) addr: u32,
    pub(crate) decoded: DecodedInst,
}

/// Instruction cache. `SIZE` must be a power of 2 so that index `(pc as usize) & (SIZE - 1)` is in bounds.
/// No Option: invalid slot is represented by CacheEntry { addr: INVALID_ADDR, .. }.
pub struct Icache<const SIZE: usize> {
    data: Box<[CacheEntry; SIZE]>,
}

impl<const SIZE: usize> Icache<SIZE> {
    /// Creates an empty I-cache. Panics if `SIZE` is not a power of 2.
    pub fn new() -> Self {
        assert!(
            SIZE > 0 && (SIZE & (SIZE - 1)) == 0,
            "Icache SIZE must be a power of 2"
        );
        Self {
            data: Box::new([CacheEntry {
                addr: INVALID_ADDR,
                decoded: DecodedInst::default(),
            }; SIZE]),
        }
    }

    #[inline(always)]
    fn index(pc: u32) -> usize {
        (pc as usize) & (SIZE - 1)
    }

    /// Returns the entry for `pc`. Caller checks entry.addr == pc for hit.
    #[inline(always)]
    pub fn get_entry_mut(&mut self, pc: u32) -> &mut CacheEntry {
        let i = Self::index(pc);
        unsafe { self.data.get_unchecked_mut(i) }
    }

    /// Clears all entries (e.g. after fence.i). Next fetch will refill.
    #[inline(never)]
    pub fn flush(&mut self) {
        for entry in self.data.iter_mut() {
            entry.addr = INVALID_ADDR;
        }
    }
}

impl<const SIZE: usize> Default for Icache<SIZE> {
    fn default() -> Self {
        Self::new()
    }
}
