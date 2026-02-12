use core::ops::Range;

use thiserror::Error;

use crate::bus::parse::parse_usize_allow_hex_underscore;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MemFault {
    #[error("invalid region '{name}': size too large to allocate on this platform: {size}")]
    SizeTooLarge { name: String, size: usize },

    #[error("invalid region '{name}': range start..end overflows usize")]
    RangeOverflow { name: String },

    #[error("invalid region '{name}': region size {size} is too small (min {min_size} bytes)")]
    RegionTooSmall {
        name: String,
        size: usize,
        min_size: usize,
    },
}

/// NOTE: `MemRegionSpec` is defined in the memory module so `BusOption` and `MemoryEntry` can
/// share the same type without importing from elsewhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemRegionSpec {
    pub name: String,
    /// Half-open address range: [start, end)
    pub region: Range<usize>,
}

impl MemRegionSpec {
    pub const MIN_REGION_SIZE: usize = 4096;

    #[inline(always)]
    pub fn base(&self) -> usize {
        self.region.start
    }

    #[inline(always)]
    pub fn size(&self) -> usize {
        self.region.end - self.region.start
    }
}

impl std::str::FromStr for MemRegionSpec {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let input = s.trim();
        if input.is_empty() {
            return Err("empty mem region spec".to_string());
        }

        let (name, rest) = input.split_once('@').ok_or_else(|| {
            "invalid mem region spec: missing '@' (expected <name>@<start>:<end>)".to_string()
        })?;

        let name = name.trim();
        if name.is_empty() {
            return Err("invalid mem region spec: empty name before '@'".to_string());
        }

        let (start_str, end_str) = rest.split_once(':').ok_or_else(|| {
            "invalid mem region spec: missing ':' (expected <name>@<start>:<end>)".to_string()
        })?;

        let start = parse_usize_allow_hex_underscore(start_str, "start")?;
        let end = parse_usize_allow_hex_underscore(end_str, "end")?;

        if end <= start {
            return Err("invalid mem region spec: end must be > start".to_string());
        }

        let size = end - start;
        if size < MemRegionSpec::MIN_REGION_SIZE {
            return Err(format!(
                "invalid mem region spec: region size {size} is too small (min {} bytes)",
                MemRegionSpec::MIN_REGION_SIZE
            ));
        }

        Ok(MemRegionSpec {
            name: name.to_string(),
            region: start..end,
        })
    }
}

/// A memory access kind (read/write), used for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessKind {
    Read,
    Write,
}

/// Page size used by D-cache; memory regions must be page-aligned and sized in whole pages.
pub const PAGE_SIZE: usize = 4096;

/// Extra bytes allocated after each region's logical size so unaligned accesses near the end
/// do not cross into the next region or OOB.
pub const REGION_TAIL_PADDING: usize = 128;

/// A contiguous RAM-backed memory region (one segment). The bus keeps a list of `MemoryEntry`
/// and uses last-hit + D-cache for fast lookup.
#[derive(Debug)]
pub struct MemoryEntry {
    pub name: String,
    pub range: Range<usize>,
    storage: Box<[u8]>,
}

impl MemoryEntry {
    /// Creates a RAM-backed region from a `MemRegionSpec`. Allocates zero-filled RAM.
    pub fn new(region: MemRegionSpec) -> Result<Self, MemFault> {
        let start = region.region.start;
        let end = region.region.end;

        if end < start {
            return Err(MemFault::RangeOverflow {
                name: region.name.clone(),
            });
        }

        let size = end - start;

        assert!(
            start % PAGE_SIZE == 0,
            "memory region '{}' start 0x{:x} must be page-aligned (page size {})",
            region.name,
            start,
            PAGE_SIZE
        );
        assert!(
            size % PAGE_SIZE == 0,
            "memory region '{}' size {} must be a multiple of page size {}",
            region.name,
            size,
            PAGE_SIZE
        );

        if size < MemRegionSpec::MIN_REGION_SIZE {
            return Err(MemFault::RegionTooSmall {
                name: region.name.clone(),
                size,
                min_size: MemRegionSpec::MIN_REGION_SIZE,
            });
        }

        let size_usize = usize::try_from(size).map_err(|_| MemFault::SizeTooLarge {
            name: region.name.clone(),
            size,
        })?;

        let storage = vec![0u8; size_usize + REGION_TAIL_PADDING].into_boxed_slice();

        Ok(Self {
            name: region.name,
            range: start..end,
            storage,
        })
    }

    #[inline(always)]
    pub fn difftest_raw_region(&mut self) -> (usize, *mut u8, usize) {
        (
            self.range.start,
            self.storage.as_mut_ptr(),
            self.range.end - self.range.start,
        )
    }

    #[inline(always)]
    pub fn difftest_raw_region_read(&self) -> (usize, *const u8, usize) {
        (
            self.range.start,
            self.storage.as_ptr(),
            self.range.end - self.range.start,
        )
    }

    #[inline(always)]
    pub fn contains(&self, range: Range<usize>) -> bool {
        (range.start >= self.range.start) && (range.end <= self.range.end)
    }

    #[inline(always)]
    pub(crate) fn ptr_at_addr(&mut self, addr: usize) -> *mut u8 {
        let off = addr - self.range.start;
        unsafe { self.storage.as_mut_ptr().add(off) }
    }

    #[inline(always)]
    pub fn read_8(&mut self, addr: usize) -> u8 {
        unsafe { *self.storage.get_unchecked(addr - self.range.start) }
    }

    #[inline(always)]
    pub fn read_16(&mut self, addr: usize) -> u16 {
        let off = addr - self.range.start;
        let p = unsafe { self.storage.as_ptr().add(off) as *const u16 };
        u16::from_le(unsafe { p.read_unaligned() })
    }

    #[inline(always)]
    pub fn read_32(&mut self, addr: usize) -> u32 {
        let off = addr - self.range.start;
        let p = unsafe { self.storage.as_ptr().add(off) as *const u32 };
        u32::from_le(unsafe { p.read_unaligned() })
    }

    #[inline(always)]
    pub fn read_64(&mut self, addr: usize) -> u64 {
        let off = addr - self.range.start;
        let p = unsafe { self.storage.as_ptr().add(off) as *const u64 };
        u64::from_le(unsafe { p.read_unaligned() })
    }

    #[inline(always)]
    pub fn read_128(&mut self, addr: usize) -> u128 {
        let off = addr - self.range.start;
        let p = unsafe { self.storage.as_ptr().add(off) as *const u128 };
        u128::from_le(unsafe { p.read_unaligned() })
    }

    #[inline(always)]
    pub fn read_bytes(&mut self, addr: usize, buf: &mut [u8]) {
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.storage.as_ptr().add(addr - self.range.start),
                buf.as_mut_ptr(),
                buf.len(),
            );
        }
    }

    #[inline(always)]
    pub fn write_8(&mut self, addr: usize, value: u8) {
        unsafe { *self.storage.get_unchecked_mut(addr - self.range.start) = value };
    }

    #[inline(always)]
    pub fn write_16(&mut self, addr: usize, value: u16) {
        let off = addr - self.range.start;
        let p = unsafe { self.storage.as_mut_ptr().add(off) as *mut u16 };
        unsafe { p.write_unaligned(value.to_le()) };
    }

    #[inline(always)]
    pub fn write_32(&mut self, addr: usize, value: u32) {
        let off = addr - self.range.start;
        let p = unsafe { self.storage.as_mut_ptr().add(off) as *mut u32 };
        unsafe { p.write_unaligned(value.to_le()) };
    }

    #[inline(always)]
    pub fn write_64(&mut self, addr: usize, value: u64) {
        let off = addr - self.range.start;
        let p = unsafe { self.storage.as_mut_ptr().add(off) as *mut u64 };
        unsafe { p.write_unaligned(value.to_le()) };
    }

    #[inline(always)]
    pub fn write_128(&mut self, addr: usize, value: u128) {
        let off = addr - self.range.start;
        let p = unsafe { self.storage.as_mut_ptr().add(off) as *mut u128 };
        unsafe { p.write_unaligned(value.to_le()) };
    }

    #[inline(always)]
    pub fn write_bytes(&mut self, addr: usize, buf: &[u8]) {
        unsafe {
            core::ptr::copy_nonoverlapping(
                buf.as_ptr(),
                self.storage.as_mut_ptr().add(addr - self.range.start),
                buf.len(),
            );
        }
    }
}
