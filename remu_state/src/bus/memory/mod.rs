remu_macro::mod_flat!(entry, dcache, elf);

use core::ops::Range;

pub use elf::try_load_elf_into_memory;
pub use entry::{AccessKind, MemFault, MemRegionSpec, MemoryEntry};

use dcache::{Dcache, PAGE_MASK, PAGE_SHIFT, PAGE_SIZE};

const DCACHE_SIZE: usize = 1 << 16;

/// Aggregates all RAM-backed regions, a D-cache, and last-hit for fast lookup.
/// ELF loading is handled here (ELF can only be loaded into memory, not devices).
pub struct Memory {
    entries: Box<[MemoryEntry]>,
    dcache: Dcache<DCACHE_SIZE>,
    last_hit: Option<usize>,
}

impl Memory {
    pub(crate) fn new(entries: Box<[MemoryEntry]>) -> Self {
        Self {
            entries,
            dcache: Dcache::new(),
            last_hit: None,
        }
    }

    pub fn entries(&self) -> &[MemoryEntry] {
        &self.entries
    }

    pub fn entries_mut(&mut self) -> &mut [MemoryEntry] {
        &mut self.entries
    }

    /// Best-effort load ELF into this memory's entries. Call after construction if desired.
    pub fn try_load_elf(
        &mut self,
        elf: &Option<std::path::PathBuf>,
        tracer: &remu_types::TracerDyn,
    ) {
        try_load_elf_into_memory(self.entries_mut(), elf, tracer);
    }

    #[inline(always)]
    fn find_memory_mut(&mut self, range: Range<usize>) -> Option<&mut MemoryEntry> {
        if let Some(i) = self.last_hit {
            if unsafe { self.entries.get_unchecked(i).contains(range.clone()) } {
                return Some(unsafe { self.entries.get_unchecked_mut(i) });
            }
        }
        self.find_memory_mut_slow(range)
    }

    #[inline(always)]
    fn find_memory_mut_slow(&mut self, range: Range<usize>) -> Option<&mut MemoryEntry> {
        for (i, m) in self.entries.iter_mut().enumerate() {
            if m.contains(range.clone()) {
                self.last_hit = Some(i);
                return Some(m);
            }
        }
        None
    }

    /// Returns pointer for `addr`; fills the page-grained cache on miss. One hit covers 4KB.
    #[inline(always)]
    fn ptr_for_addr(&mut self, addr: usize) -> Option<*mut u8> {
        let page = addr >> PAGE_SHIFT;
        let offset = addr & PAGE_MASK;
        {
            let entry = self.dcache.get_entry_mut(addr);
            if entry.tag == page {
                return Some(unsafe { entry.base_ptr.add(offset) });
            }
        }
        let page_start = addr & !PAGE_MASK;
        let m = self.find_memory_mut(page_start..page_start + PAGE_SIZE)?;
        let base_ptr = m.ptr_at_addr(page_start);
        let entry = self.dcache.get_entry_mut(addr);
        entry.tag = page;
        entry.base_ptr = base_ptr;
        Some(unsafe { base_ptr.add(offset) })
    }

    #[inline(always)]
    pub(crate) fn read_8(&mut self, addr: usize) -> Option<u8> {
        let ptr = self.ptr_for_addr(addr)?;
        Some(unsafe { *ptr })
    }

    #[inline(always)]
    pub(crate) fn read_16(&mut self, addr: usize) -> Option<u16> {
        let ptr = self.ptr_for_addr(addr)?;
        let raw = unsafe { (ptr as *const u16).read_unaligned() };
        Some(u16::from_le(raw))
    }

    #[inline(always)]
    pub(crate) fn read_32(&mut self, addr: usize) -> Option<u32> {
        let ptr = self.ptr_for_addr(addr)?;
        let raw = unsafe { (ptr as *const u32).read_unaligned() };
        Some(u32::from_le(raw))
    }

    #[inline(always)]
    pub(crate) fn read_64(&mut self, addr: usize) -> Option<u64> {
        let ptr = self.ptr_for_addr(addr)?;
        let raw = unsafe { (ptr as *const u64).read_unaligned() };
        Some(u64::from_le(raw))
    }

    #[inline(always)]
    pub(crate) fn read_128(&mut self, addr: usize) -> Option<u128> {
        let ptr = self.ptr_for_addr(addr)?;
        let raw = unsafe { (ptr as *const u128).read_unaligned() };
        Some(u128::from_le(raw))
    }

    #[inline(always)]
    pub(crate) fn read_bytes(&mut self, addr: usize, buf: &mut [u8]) -> Option<()> {
        let m = self.find_memory_mut(addr..addr + buf.len())?;
        unsafe {
            core::ptr::copy_nonoverlapping(m.ptr_at_addr(addr), buf.as_mut_ptr(), buf.len());
        }
        Some(())
    }

    #[inline(always)]
    pub(crate) fn write_8(&mut self, addr: usize, value: u8) -> Option<()> {
        let ptr = self.ptr_for_addr(addr)?;
        unsafe { *ptr = value };
        Some(())
    }

    #[inline(always)]
    pub(crate) fn write_16(&mut self, addr: usize, value: u16) -> Option<()> {
        let ptr = self.ptr_for_addr(addr)?;
        unsafe { (ptr as *mut u16).write_unaligned(value.to_le()) };
        Some(())
    }

    #[inline(always)]
    pub(crate) fn write_32(&mut self, addr: usize, value: u32) -> Option<()> {
        let ptr = self.ptr_for_addr(addr)?;
        unsafe { (ptr as *mut u32).write_unaligned(value.to_le()) };
        Some(())
    }

    #[inline(always)]
    pub(crate) fn write_64(&mut self, addr: usize, value: u64) -> Option<()> {
        let ptr = self.ptr_for_addr(addr)?;
        unsafe { (ptr as *mut u64).write_unaligned(value.to_le()) };
        Some(())
    }

    #[inline(always)]
    pub(crate) fn write_128(&mut self, addr: usize, value: u128) -> Option<()> {
        let ptr = self.ptr_for_addr(addr)?;
        unsafe { (ptr as *mut u128).write_unaligned(value.to_le()) };
        Some(())
    }

    #[inline(always)]
    pub(crate) fn write_bytes(&mut self, addr: usize, buf: &[u8]) -> Option<()> {
        let m = self.find_memory_mut(addr..addr + buf.len())?;
        unsafe {
            core::ptr::copy_nonoverlapping(buf.as_ptr(), m.ptr_at_addr(addr), buf.len());
        }
        Some(())
    }
}
