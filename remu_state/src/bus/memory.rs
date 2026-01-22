use core::ops::Range;

use thiserror::Error;

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

/// NOTE: `MemRegionSpec` is currently defined in this module so `BusOption` and `Memory` can share
/// the same type without importing it from elsewhere.
///
/// If you later want to move option parsing into another crate, you can re-export this type (or a
/// separate spec type) from a shared crate, and keep `Memory::new(spec)` unchanged.
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

fn parse_usize_allow_hex_underscore(s: &str, field: &str) -> Result<usize, String> {
    let raw = s.trim();
    if raw.is_empty() {
        return Err(format!("invalid mem region spec: empty {}", field));
    }

    // allow '_' inside numbers
    let cleaned: String = raw.chars().filter(|&c| c != '_').collect();

    // Accept:
    // - 0x... / 0X... hex
    // - bare hex (e.g. "8000_0000")
    // - decimal (digits only)
    let value = if let Some(hex) = cleaned
        .strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
    {
        usize::from_str_radix(hex, 16).map_err(|e| {
            format!(
                "invalid mem region spec: {} '{}' is not valid hex: {}",
                field, raw, e
            )
        })?
    } else if cleaned.chars().all(|c| c.is_ascii_digit()) {
        cleaned.parse::<usize>().map_err(|e| {
            format!(
                "invalid mem region spec: {} '{}' is not valid decimal: {}",
                field, raw, e
            )
        })?
    } else {
        usize::from_str_radix(&cleaned, 16).map_err(|e| {
            format!(
                "invalid mem region spec: {} '{}' is not valid hex: {}",
                field, raw, e
            )
        })?
    };

    Ok(value)
}

impl std::str::FromStr for MemRegionSpec {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Expected format: "<name>@<start>:<end>"
        // Example: "ram1@0x8000_0000:0x8800_0000"
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

/// A contiguous RAM-backed memory region.
///
/// This is a *region* (segment). Higher-level bus/address-space code can keep a `Vec<Memory>`
/// and perform fast region lookup (binary search, page table, last-hit cache, etc.).
#[derive(Debug)]
pub struct Memory {
    pub name: String,
    pub range: Range<usize>,
    storage: Box<[u8]>,
}

impl Memory {
    /// Create a RAM-backed region from a `MemRegionSpec`.
    ///
    /// Allocates `size` bytes of zero-filled RAM.
    ///
    /// NOTE: Multi-byte reads/writes are currently always little-endian.
    pub fn new(region: MemRegionSpec) -> Result<Self, MemFault> {
        let start = region.region.start;
        let end = region.region.end;

        if end < start {
            // This should be impossible with parsing, but keep it defensive.
            return Err(MemFault::RangeOverflow {
                name: region.name.clone(),
            });
        }

        let size = end - start;

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

        // NOTE: this allocates and zero-fills. If you later want faster init for huge RAM,
        // consider an mmap-backed implementation.
        let storage = vec![0u8; size_usize].into_boxed_slice();

        Ok(Self {
            name: region.name,
            range: start..end,
            storage,
        })
    }

    /// Returns whether `addr` is contained in this region.
    #[inline(always)]
    pub fn contains(&self, range: Range<usize>) -> bool {
        (range.start >= self.range.start) && (range.end <= self.range.end)
    }

    #[inline(always)]
    pub fn read_8(&mut self, addr: usize) -> u8 {
        unsafe { *self.storage.get_unchecked(addr - self.range.start) }
    }

    #[inline(always)]
    pub fn read_16(&mut self, addr: usize) -> u16 {
        // SAFETY: `checked_range_rel` guarantees `[r.start, r.start+2)` is in-bounds.
        // We keep correct unaligned semantics, but add a faster aligned path.
        let off = addr - self.range.start;
        let p = unsafe { self.storage.as_ptr().add(off) as *const u16 };
        let raw = if (off & 1) == 0 {
            // SAFETY: `p` is in-bounds and properly aligned when `off` is 2-byte aligned.
            unsafe { p.read() }
        } else {
            unsafe { p.read_unaligned() }
        };
        u16::from_le(raw)
    }

    #[inline(always)]
    pub fn read_32(&mut self, addr: usize) -> u32 {
        // SAFETY: `checked_range_rel` guarantees `[r.start, r.start+4)` is in-bounds.
        // We keep correct unaligned semantics, but add a faster aligned path.
        let off = addr - self.range.start;
        let p = unsafe { self.storage.as_ptr().add(off) as *const u32 };
        let raw = if (off & 3) == 0 {
            // SAFETY: `p` is in-bounds and properly aligned when `off` is 4-byte aligned.
            unsafe { p.read() }
        } else {
            unsafe { p.read_unaligned() }
        };
        u32::from_le(raw)
    }

    #[inline(always)]
    pub fn read_64(&mut self, addr: usize) -> u64 {
        // SAFETY: `checked_range_rel` guarantees `[r.start, r.start+8)` is in-bounds.
        // We keep correct unaligned semantics, but add a faster aligned path.
        let off = addr - self.range.start;
        let p = unsafe { self.storage.as_ptr().add(off) as *const u64 };
        let raw = if (off & 7) == 0 {
            // SAFETY: `p` is in-bounds and properly aligned when `off` is 8-byte aligned.
            unsafe { p.read() }
        } else {
            unsafe { p.read_unaligned() }
        };
        u64::from_le(raw)
    }

    #[inline(always)]
    pub fn read_128(&mut self, addr: usize) -> u128 {
        // SAFETY: `checked_range_rel` guarantees `[r.start, r.start+16)` is in-bounds.
        // We keep correct unaligned semantics, but add a faster aligned path.
        let off = addr - self.range.start;
        let p = unsafe { self.storage.as_ptr().add(off) as *const u128 };
        let raw = if (off & 15) == 0 {
            // SAFETY: `p` is in-bounds and properly aligned when `off` is 16-byte aligned.
            unsafe { p.read() }
        } else {
            unsafe { p.read_unaligned() }
        };
        u128::from_le(raw)
    }

    #[inline(always)]
    pub fn read_bytes(&mut self, addr: usize, buf: &mut [u8]) {
        // SAFETY:
        // - `checked_range_rel` guarantees `[r.start, r.start + buf.len())` is in-bounds.
        // - `buf` is a valid writable slice of length `buf.len()`.
        // - Source and destination do not overlap (storage and `buf` are distinct allocations).
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
        unsafe {
            *self.storage.get_unchecked_mut(addr - self.range.start) = value;
        }
    }

    #[inline(always)]
    pub fn write_16(&mut self, addr: usize, value: u16) {
        // SAFETY: `checked_range_rel` guarantees `[r.start, r.start+2)` is in-bounds.
        // We keep correct unaligned semantics, but add a faster aligned path.
        let off = addr - self.range.start;
        let p = unsafe { self.storage.as_mut_ptr().add(off) as *mut u16 };
        let le = value.to_le();
        if (off & 1) == 0 {
            // SAFETY: `p` is in-bounds and properly aligned when `off` is 2-byte aligned.
            unsafe { p.write(le) };
        } else {
            unsafe { p.write_unaligned(le) };
        }
    }

    #[inline(always)]
    pub fn write_32(&mut self, addr: usize, value: u32) {
        // SAFETY: `checked_range_rel` guarantees `[r.start, r.start+4)` is in-bounds.
        // We keep correct unaligned semantics, but add a faster aligned path.
        let off = addr - self.range.start;
        let p = unsafe { self.storage.as_mut_ptr().add(off) as *mut u32 };
        let le = value.to_le();
        if (off & 3) == 0 {
            // SAFETY: `p` is in-bounds and properly aligned when `off` is 4-byte aligned.
            unsafe { p.write(le) };
        } else {
            unsafe { p.write_unaligned(le) };
        }
    }

    #[inline(always)]
    pub fn write_64(&mut self, addr: usize, value: u64) {
        // SAFETY: `checked_range_rel` guarantees `[r.start, r.start+8)` is in-bounds.
        // We keep correct unaligned semantics, but add a faster aligned path.
        let off = addr - self.range.start;
        let p = unsafe { self.storage.as_mut_ptr().add(off) as *mut u64 };
        let le = value.to_le();
        if (off & 7) == 0 {
            // SAFETY: `p` is in-bounds and properly aligned when `off` is 8-byte aligned.
            unsafe { p.write(le) };
        } else {
            unsafe { p.write_unaligned(le) };
        }
    }

    #[inline(always)]
    pub fn write_128(&mut self, addr: usize, value: u128) {
        // SAFETY: `checked_range_rel` guarantees `[r.start, r.start+16)` is in-bounds.
        // We keep correct unaligned semantics, but add a faster aligned path.
        let off = addr - self.range.start;
        let p = unsafe { self.storage.as_mut_ptr().add(off) as *mut u128 };
        let le = value.to_le();
        if (off & 15) == 0 {
            // SAFETY: `p` is in-bounds and properly aligned when `off` is 16-byte aligned.
            unsafe { p.write(le) };
        } else {
            unsafe { p.write_unaligned(le) };
        }
    }

    #[inline(always)]
    pub fn write_bytes(&mut self, addr: usize, buf: &[u8]) {
        // SAFETY:
        // - `checked_range_rel` guarantees `[r.start, r.start + buf.len())` is in-bounds.
        // - `buf` is a valid readable slice of length `buf.len()`.
        // - Source and destination do not overlap (storage and `buf` are distinct allocations).
        unsafe {
            core::ptr::copy_nonoverlapping(
                buf.as_ptr(),
                self.storage.as_mut_ptr().add(addr - self.range.start),
                buf.len(),
            );
        }
    }
}
