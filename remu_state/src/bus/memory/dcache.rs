//! Software TLB (Addend-style): page-grained D-cache. Hot path: tag check + (addr + addend).
//! Cold path (refill) is out-of-line to avoid I-cache pollution.

pub(crate) const PAGE_SHIFT: usize = 8;
pub(crate) const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
pub(crate) const PAGE_MASK: usize = PAGE_SIZE - 1;

/// Sentinel for empty slot. Real page numbers are never this (no guest has page usize::MAX).
pub(crate) const INVALID_TAG: usize = usize::MAX;

/// One cache line: VPN (tag) + addend. Host pointer for guest `addr` = `addr.wrapping_add(addend)`.
/// Addend = host_page_base - guest_page_base, so one ADD on hit (no AND for offset).
#[derive(Clone, Copy)]
#[repr(align(16))]
pub(crate) struct DcacheEntry {
    pub(crate) tag: usize,
    pub(crate) addend: usize,
}

/// Page-grained D-cache. Index = (addr >> PAGE_SHIFT) & (SIZE - 1). SIZE must be a power of 2.
pub(crate) struct Dcache<const SIZE: usize> {
    data: Box<[DcacheEntry; SIZE]>,
}

impl<const SIZE: usize> Dcache<SIZE> {
    pub(crate) fn new() -> Self {
        assert!(
            SIZE > 0 && (SIZE & (SIZE - 1)) == 0,
            "Dcache SIZE must be a power of 2"
        );
        Self {
            data: Box::new(
                [DcacheEntry {
                    tag: INVALID_TAG,
                    addend: 0,
                }; SIZE],
            ),
        }
    }

    #[inline(always)]
    pub(crate) fn index(addr: usize) -> usize {
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
