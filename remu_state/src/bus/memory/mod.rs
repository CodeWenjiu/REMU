remu_macro::mod_flat!(entry, dcache, elf);

use core::ops::Range;

pub use elf::try_load_elf_into_memory;
pub use entry::{AccessKind, MemFault, MemRegionSpec, MemoryEntry};

use dcache::{Dcache, PAGE_MASK, PAGE_SHIFT};

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

    #[inline(never)]
    fn find_memory_mut_slow(&mut self, range: Range<usize>) -> Option<&mut MemoryEntry> {
        for (i, m) in self.entries.iter_mut().enumerate() {
            if m.contains(range.clone()) {
                self.last_hit = Some(i);
                return Some(m);
            }
        }
        None
    }

    /// Refill D-cache for the page containing `addr`. Returns addend or None if unmapped.
    #[inline(never)]
    fn refill_dcache(&mut self, addr: usize) -> Option<usize> {
        let page_start = addr & !PAGE_MASK;
        let m = self.find_memory_mut(page_start..page_start + dcache::PAGE_SIZE)?;
        let host_base = m.ptr_at_addr(page_start) as usize;
        let addend = host_base.wrapping_sub(page_start);
        let entry = self.dcache.get_entry_mut(addr);
        entry.tag = addr >> PAGE_SHIFT;
        entry.addend = addend;
        Some(addend)
    }

    #[inline(always)]
    pub(crate) fn read_8(&mut self, addr: usize) -> Option<u8> {
        let entry = self.dcache.get_entry_mut(addr);
        if entry.tag == (addr >> PAGE_SHIFT) {
            let host_ptr = addr.wrapping_add(entry.addend) as *const u8;
            return Some(unsafe { *host_ptr });
        }
        self.read_8_slow(addr)
    }

    #[inline(never)]
    fn read_8_slow(&mut self, addr: usize) -> Option<u8> {
        let addend = self.refill_dcache(addr)?;
        let host_ptr = addr.wrapping_add(addend) as *const u8;
        Some(unsafe { *host_ptr })
    }

    #[inline(always)]
    pub(crate) fn read_16(&mut self, addr: usize) -> Option<u16> {
        let entry = self.dcache.get_entry_mut(addr);
        if entry.tag == (addr >> PAGE_SHIFT) {
            let host_ptr = addr.wrapping_add(entry.addend) as *const u16;
            return Some(unsafe { host_ptr.read_unaligned() }.to_le());
        }
        self.read_16_slow(addr)
    }

    #[inline(never)]
    fn read_16_slow(&mut self, addr: usize) -> Option<u16> {
        let addend = self.refill_dcache(addr)?;
        let host_ptr = addr.wrapping_add(addend) as *const u16;
        Some(unsafe { host_ptr.read_unaligned() }.to_le())
    }

    #[inline(always)]
    pub(crate) fn read_32(&mut self, addr: usize) -> Option<u32> {
        let entry = self.dcache.get_entry_mut(addr);
        if entry.tag == (addr >> PAGE_SHIFT) {
            let host_ptr = addr.wrapping_add(entry.addend) as *const u32;
            return Some(unsafe { host_ptr.read_unaligned() }.to_le());
        }
        self.read_32_slow(addr)
    }

    #[inline(never)]
    fn read_32_slow(&mut self, addr: usize) -> Option<u32> {
        let addend = self.refill_dcache(addr)?;
        let host_ptr = addr.wrapping_add(addend) as *const u32;
        Some(unsafe { host_ptr.read_unaligned() }.to_le())
    }

    #[inline(always)]
    pub(crate) fn read_64(&mut self, addr: usize) -> Option<u64> {
        let entry = self.dcache.get_entry_mut(addr);
        if entry.tag == (addr >> PAGE_SHIFT) {
            let host_ptr = addr.wrapping_add(entry.addend) as *const u64;
            return Some(unsafe { host_ptr.read_unaligned() }.to_le());
        }
        self.read_64_slow(addr)
    }

    #[inline(never)]
    fn read_64_slow(&mut self, addr: usize) -> Option<u64> {
        let addend = self.refill_dcache(addr)?;
        let host_ptr = addr.wrapping_add(addend) as *const u64;
        Some(unsafe { host_ptr.read_unaligned() }.to_le())
    }

    #[inline(always)]
    pub(crate) fn read_128(&mut self, addr: usize) -> Option<u128> {
        let entry = self.dcache.get_entry_mut(addr);
        if entry.tag == (addr >> PAGE_SHIFT) {
            let host_ptr = addr.wrapping_add(entry.addend) as *const u128;
            return Some(unsafe { host_ptr.read_unaligned() }.to_le());
        }
        self.read_128_slow(addr)
    }

    #[inline(never)]
    fn read_128_slow(&mut self, addr: usize) -> Option<u128> {
        let addend = self.refill_dcache(addr)?;
        let host_ptr = addr.wrapping_add(addend) as *const u128;
        Some(unsafe { host_ptr.read_unaligned() }.to_le())
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
        let entry = self.dcache.get_entry_mut(addr);
        if entry.tag == (addr >> PAGE_SHIFT) {
            let host_ptr = addr.wrapping_add(entry.addend) as *mut u8;
            unsafe { *host_ptr = value };
            return Some(());
        }
        self.write_8_slow(addr, value)
    }

    #[inline(never)]
    fn write_8_slow(&mut self, addr: usize, value: u8) -> Option<()> {
        let addend = self.refill_dcache(addr)?;
        let host_ptr = addr.wrapping_add(addend) as *mut u8;
        unsafe { *host_ptr = value };
        Some(())
    }

    #[inline(always)]
    pub(crate) fn write_16(&mut self, addr: usize, value: u16) -> Option<()> {
        let entry = self.dcache.get_entry_mut(addr);
        if entry.tag == (addr >> PAGE_SHIFT) {
            let host_ptr = addr.wrapping_add(entry.addend) as *mut u16;
            unsafe { host_ptr.write_unaligned(value.to_le()) };
            return Some(());
        }
        self.write_16_slow(addr, value)
    }

    #[inline(never)]
    fn write_16_slow(&mut self, addr: usize, value: u16) -> Option<()> {
        let addend = self.refill_dcache(addr)?;
        let host_ptr = addr.wrapping_add(addend) as *mut u16;
        unsafe { host_ptr.write_unaligned(value.to_le()) };
        Some(())
    }

    #[inline(always)]
    pub(crate) fn write_32(&mut self, addr: usize, value: u32) -> Option<()> {
        let entry = self.dcache.get_entry_mut(addr);
        if entry.tag == (addr >> PAGE_SHIFT) {
            let host_ptr = addr.wrapping_add(entry.addend) as *mut u32;
            unsafe { host_ptr.write_unaligned(value.to_le()) };
            return Some(());
        }
        self.write_32_slow(addr, value)
    }

    #[inline(never)]
    fn write_32_slow(&mut self, addr: usize, value: u32) -> Option<()> {
        let addend = self.refill_dcache(addr)?;
        let host_ptr = addr.wrapping_add(addend) as *mut u32;
        unsafe { host_ptr.write_unaligned(value.to_le()) };
        Some(())
    }

    #[inline(always)]
    pub(crate) fn write_64(&mut self, addr: usize, value: u64) -> Option<()> {
        let entry = self.dcache.get_entry_mut(addr);
        if entry.tag == (addr >> PAGE_SHIFT) {
            let host_ptr = addr.wrapping_add(entry.addend) as *mut u64;
            unsafe { host_ptr.write_unaligned(value.to_le()) };
            return Some(());
        }
        self.write_64_slow(addr, value)
    }

    #[inline(never)]
    fn write_64_slow(&mut self, addr: usize, value: u64) -> Option<()> {
        let addend = self.refill_dcache(addr)?;
        let host_ptr = addr.wrapping_add(addend) as *mut u64;
        unsafe { host_ptr.write_unaligned(value.to_le()) };
        Some(())
    }

    #[inline(always)]
    pub(crate) fn write_128(&mut self, addr: usize, value: u128) -> Option<()> {
        let entry = self.dcache.get_entry_mut(addr);
        if entry.tag == (addr >> PAGE_SHIFT) {
            let host_ptr = addr.wrapping_add(entry.addend) as *mut u128;
            unsafe { host_ptr.write_unaligned(value.to_le()) };
            return Some(());
        }
        self.write_128_slow(addr, value)
    }

    #[inline(never)]
    fn write_128_slow(&mut self, addr: usize, value: u128) -> Option<()> {
        let addend = self.refill_dcache(addr)?;
        let host_ptr = addr.wrapping_add(addend) as *mut u128;
        unsafe { host_ptr.write_unaligned(value.to_le()) };
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
