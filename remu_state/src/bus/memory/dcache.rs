//! Software TLB (Addend-style): page-grained D-cache, no Option, one hit covers 4KB.

pub const PAGE_SHIFT: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
pub const PAGE_MASK: usize = PAGE_SIZE - 1;

/// Sentinel for empty slot. Real page numbers are never this (no guest has page usize::MAX).
pub(crate) const INVALID_TAG: usize = usize::MAX;

/// One cache line: page tag + base pointer for that page. No Option; invalid = tag == INVALID_TAG.
#[derive(Clone, Copy)]
pub(crate) struct DcacheEntry {
    pub(crate) tag: usize,
    pub(crate) base_ptr: *mut u8,
}

/// Page-grained D-cache. Index = (addr >> PAGE_SHIFT) & (SIZE - 1). SIZE must be a power of 2.
pub(crate) struct Dcache<const SIZE: usize> {
    data: Box<[DcacheEntry; SIZE]>,
}

impl<const SIZE: usize> Dcache<SIZE> {
    pub(crate) fn new() -> Self {
        debug_assert!(
            SIZE > 0 && (SIZE & (SIZE - 1)) == 0,
            "Dcache SIZE must be a power of 2"
        );
        Self {
            data: Box::new([DcacheEntry {
                tag: INVALID_TAG,
                base_ptr: core::ptr::null_mut(),
            }; SIZE]),
        }
    }

    #[inline(always)]
    fn index(addr: usize) -> usize {
        (addr >> PAGE_SHIFT) & (SIZE - 1)
    }

    /// Returns the entry for the page containing `addr`. Caller checks tag and fills on miss.
    #[inline(always)]
    pub(crate) fn get_entry_mut(&mut self, addr: usize) -> &mut DcacheEntry {
        let i = Self::index(addr);
        unsafe { self.data.get_unchecked_mut(i) }
    }
}

impl<const SIZE: usize> Default for Dcache<SIZE> {
    fn default() -> Self {
        Self::new()
    }
}
