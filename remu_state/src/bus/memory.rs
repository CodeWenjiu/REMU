use core::ops::Range;

use thiserror::Error;

use crate::bus::BusAccess;

/// NOTE: `MemRegionSpec` is currently defined in this module so `BusOption` and `Memory` can share
/// the same type without importing it from elsewhere.
///
/// If you later want to move option parsing into another crate, you can re-export this type (or a
/// separate spec type) from a shared crate, and keep `Memory::new(spec)` unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemRegionSpec {
    pub name: String,
    pub base: usize,
    pub size: usize,
}

impl std::str::FromStr for MemRegionSpec {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Expected format: "<name>@<base>:<size>"
        // Example: "ram1@0x8000_0000:0x0800_0000"
        let input = s.trim();
        if input.is_empty() {
            return Err("empty mem region spec".to_string());
        }

        let (name, rest) = input.split_once('@').ok_or_else(|| {
            "invalid mem region spec: missing '@' (expected <name>@<base>:<size>)".to_string()
        })?;

        let name = name.trim();
        if name.is_empty() {
            return Err("invalid mem region spec: empty name before '@'".to_string());
        }

        let (base_str, size_str) = rest.split_once(':').ok_or_else(|| {
            "invalid mem region spec: missing ':' (expected <name>@<base>:<size>)".to_string()
        })?;

        fn parse_usize_allow_hex_underscore(s: &str, field: &str) -> Result<usize, String> {
            let raw = s.trim();
            if raw.is_empty() {
                return Err(format!("invalid mem region spec: empty {}", field));
            }

            // allow '_' inside hex numbers
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

        let base = parse_usize_allow_hex_underscore(base_str, "base")?;
        let size = parse_usize_allow_hex_underscore(size_str, "size")?;

        if size == 0 {
            return Err("invalid mem region spec: size must be > 0".to_string());
        }

        Ok(MemRegionSpec {
            name: name.to_string(),
            base,
            size,
        })
    }
}

/// A memory access kind (read/write), used for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessKind {
    Read,
    Write,
}

/// In-memory fault type returned by RAM-backed `Memory` operations.
///
/// This is intentionally ISA-agnostic. The simulator/CPU layer should map it to an ISA trap.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MemFault {
    #[error("unmapped address: 0x{addr:016x}")]
    Unmapped { addr: usize },

    #[error(
        "out of bounds {kind:?} at 0x{addr:016x} (size={size}) for region '{region}' \
         [0x{base:016x}..0x{end:016x})"
    )]
    OutOfBounds {
        kind: AccessKind,
        addr: usize,
        size: usize,
        region: String,
        base: usize,
        end: usize,
    },

    #[error("invalid region '{name}': size too large to allocate on this platform: {size}")]
    SizeTooLarge { name: String, size: usize },

    #[error("invalid region '{name}': base+size overflows u64")]
    RangeOverflow { name: String },
}

/// A contiguous RAM-backed memory region.
///
/// This is a *region* (segment). Higher-level bus/address-space code can keep a `Vec<Memory>`
/// and perform fast region lookup (binary search, page table, last-hit cache, etc.).
#[derive(Debug)]
pub struct Memory {
    pub name: String,
    pub base: usize,
    pub end: usize, // exclusive
    storage: Vec<u8>,
}

impl Memory {
    /// Create a RAM-backed region from a `MemRegionSpec`.
    ///
    /// Allocates `size` bytes of zero-filled RAM.
    ///
    /// NOTE: Multi-byte reads/writes are currently always little-endian.
    pub fn new(region: MemRegionSpec) -> Result<Self, MemFault> {
        let end = region
            .base
            .checked_add(region.size)
            .ok_or_else(|| MemFault::RangeOverflow {
                name: region.name.clone(),
            })?;

        let size_usize = usize::try_from(region.size).map_err(|_| MemFault::SizeTooLarge {
            name: region.name.clone(),
            size: region.size,
        })?;

        // NOTE: this allocates and zero-fills. If you later want faster init for huge RAM,
        // consider an mmap-backed implementation.
        let storage = vec![0u8; size_usize];

        Ok(Self {
            name: region.name,
            base: region.base,
            end,
            storage,
        })
    }

    /// Returns whether `addr` is contained in this region.
    #[inline(always)]
    pub fn contains(&self, addr: usize) -> bool {
        self.base <= addr && addr < self.end
    }

    /// Convert a **relative** address (offset from region base) to a storage offset (usize),
    /// returning a bound-checked range.
    ///
    /// Callers that deal in absolute addresses should translate first:
    /// `let rel = addr - self.base`.
    #[inline(always)]
    fn checked_range_rel(
        &self,
        kind: AccessKind,
        rel_addr: usize,
        size: usize,
    ) -> Result<Range<usize>, MemFault> {
        #[cold]
        #[inline(never)]
        fn oob(mem: &Memory, kind: AccessKind, rel_addr: usize, size: usize) -> MemFault {
            MemFault::OutOfBounds {
                kind,
                addr: mem.base.wrapping_add(rel_addr),
                size,
                region: mem.name.clone(),
                base: mem.base,
                end: mem.end,
            }
        }

        let region_size = self.storage.len();

        // Fast path: avoid checked_add and keep error construction out of line.
        //
        // We want: rel_addr + size <= region_size, without risking overflow.
        // This is equivalent to: size <= region_size && rel_addr <= region_size - size
        if size > region_size {
            return Err(oob(self, kind, rel_addr, size));
        }
        let max_off = region_size - size;
        if rel_addr > max_off {
            return Err(oob(self, kind, rel_addr, size));
        }

        let off = rel_addr;
        Ok(off..(off + size))
    }
}

impl BusAccess for Memory {
    type Fault = MemFault;

    #[inline(always)]
    fn read_8(&mut self, addr: usize) -> Result<u8, Box<Self::Fault>> {
        let r = self
            .checked_range_rel(AccessKind::Read, addr, 1)
            .map_err(Box::new)?;
        Ok(self.storage[r.start])
    }

    #[inline(always)]
    fn read_16(&mut self, addr: usize) -> Result<u16, Box<Self::Fault>> {
        let r = self
            .checked_range_rel(AccessKind::Read, addr, 2)
            .map_err(Box::new)?;

        // SAFETY: `checked_range_rel` guarantees `[r.start, r.start+2)` is in-bounds.
        // We keep correct unaligned semantics, but add a faster aligned path.
        let off = r.start;
        let p = unsafe { self.storage.as_ptr().add(off) as *const u16 };
        let raw = if (off & 1) == 0 {
            // SAFETY: `p` is in-bounds and properly aligned when `off` is 2-byte aligned.
            unsafe { p.read() }
        } else {
            unsafe { p.read_unaligned() }
        };
        Ok(u16::from_le(raw))
    }

    #[inline(always)]
    fn read_32(&mut self, addr: usize) -> Result<u32, Box<Self::Fault>> {
        let r = self
            .checked_range_rel(AccessKind::Read, addr, 4)
            .map_err(Box::new)?;

        // SAFETY: `checked_range_rel` guarantees `[r.start, r.start+4)` is in-bounds.
        // We keep correct unaligned semantics, but add a faster aligned path.
        let off = r.start;
        let p = unsafe { self.storage.as_ptr().add(off) as *const u32 };
        let raw = if (off & 3) == 0 {
            // SAFETY: `p` is in-bounds and properly aligned when `off` is 4-byte aligned.
            unsafe { p.read() }
        } else {
            unsafe { p.read_unaligned() }
        };
        Ok(u32::from_le(raw))
    }

    #[inline(always)]
    fn read_64(&mut self, addr: usize) -> Result<u64, Box<Self::Fault>> {
        let r = self
            .checked_range_rel(AccessKind::Read, addr, 8)
            .map_err(Box::new)?;

        // SAFETY: `checked_range_rel` guarantees `[r.start, r.start+8)` is in-bounds.
        // We keep correct unaligned semantics, but add a faster aligned path.
        let off = r.start;
        let p = unsafe { self.storage.as_ptr().add(off) as *const u64 };
        let raw = if (off & 7) == 0 {
            // SAFETY: `p` is in-bounds and properly aligned when `off` is 8-byte aligned.
            unsafe { p.read() }
        } else {
            unsafe { p.read_unaligned() }
        };
        Ok(u64::from_le(raw))
    }

    #[inline(always)]
    fn read_128(&mut self, addr: usize) -> Result<u128, Box<Self::Fault>> {
        let r = self
            .checked_range_rel(AccessKind::Read, addr, 16)
            .map_err(Box::new)?;

        // SAFETY: `checked_range_rel` guarantees `[r.start, r.start+16)` is in-bounds.
        // We keep correct unaligned semantics, but add a faster aligned path.
        let off = r.start;
        let p = unsafe { self.storage.as_ptr().add(off) as *const u128 };
        let raw = if (off & 15) == 0 {
            // SAFETY: `p` is in-bounds and properly aligned when `off` is 16-byte aligned.
            unsafe { p.read() }
        } else {
            unsafe { p.read_unaligned() }
        };
        Ok(u128::from_le(raw))
    }

    #[inline(always)]
    fn read_bytes(&mut self, addr: usize, buf: &mut [u8]) -> Result<(), Box<Self::Fault>> {
        let r = self
            .checked_range_rel(AccessKind::Read, addr, buf.len())
            .map_err(Box::new)?;

        // SAFETY:
        // - `checked_range_rel` guarantees `[r.start, r.start + buf.len())` is in-bounds.
        // - `buf` is a valid writable slice of length `buf.len()`.
        // - Source and destination do not overlap (storage and `buf` are distinct allocations).
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.storage.as_ptr().add(r.start),
                buf.as_mut_ptr(),
                buf.len(),
            );
        }
        Ok(())
    }

    #[inline(always)]
    fn write_8(&mut self, addr: usize, value: u8) -> Result<(), Box<Self::Fault>> {
        let r = self
            .checked_range_rel(AccessKind::Write, addr, 1)
            .map_err(Box::new)?;
        self.storage[r.start] = value;
        Ok(())
    }

    #[inline(always)]
    fn write_16(&mut self, addr: usize, value: u16) -> Result<(), Box<Self::Fault>> {
        let r = self
            .checked_range_rel(AccessKind::Write, addr, 2)
            .map_err(Box::new)?;

        // SAFETY: `checked_range_rel` guarantees `[r.start, r.start+2)` is in-bounds.
        // We keep correct unaligned semantics, but add a faster aligned path.
        let off = r.start;
        let p = unsafe { self.storage.as_mut_ptr().add(off) as *mut u16 };
        let le = value.to_le();
        if (off & 1) == 0 {
            // SAFETY: `p` is in-bounds and properly aligned when `off` is 2-byte aligned.
            unsafe { p.write(le) };
        } else {
            unsafe { p.write_unaligned(le) };
        }
        Ok(())
    }

    #[inline(always)]
    fn write_32(&mut self, addr: usize, value: u32) -> Result<(), Box<Self::Fault>> {
        let r = self
            .checked_range_rel(AccessKind::Write, addr, 4)
            .map_err(Box::new)?;

        // SAFETY: `checked_range_rel` guarantees `[r.start, r.start+4)` is in-bounds.
        // We keep correct unaligned semantics, but add a faster aligned path.
        let off = r.start;
        let p = unsafe { self.storage.as_mut_ptr().add(off) as *mut u32 };
        let le = value.to_le();
        if (off & 3) == 0 {
            // SAFETY: `p` is in-bounds and properly aligned when `off` is 4-byte aligned.
            unsafe { p.write(le) };
        } else {
            unsafe { p.write_unaligned(le) };
        }
        Ok(())
    }

    #[inline(always)]
    fn write_64(&mut self, addr: usize, value: u64) -> Result<(), Box<Self::Fault>> {
        let r = self
            .checked_range_rel(AccessKind::Write, addr, 8)
            .map_err(Box::new)?;

        // SAFETY: `checked_range_rel` guarantees `[r.start, r.start+8)` is in-bounds.
        // We keep correct unaligned semantics, but add a faster aligned path.
        let off = r.start;
        let p = unsafe { self.storage.as_mut_ptr().add(off) as *mut u64 };
        let le = value.to_le();
        if (off & 7) == 0 {
            // SAFETY: `p` is in-bounds and properly aligned when `off` is 8-byte aligned.
            unsafe { p.write(le) };
        } else {
            unsafe { p.write_unaligned(le) };
        }
        Ok(())
    }

    #[inline(always)]
    fn write_128(&mut self, addr: usize, value: u128) -> Result<(), Box<Self::Fault>> {
        let r = self
            .checked_range_rel(AccessKind::Write, addr, 16)
            .map_err(Box::new)?;

        // SAFETY: `checked_range_rel` guarantees `[r.start, r.start+16)` is in-bounds.
        // We keep correct unaligned semantics, but add a faster aligned path.
        let off = r.start;
        let p = unsafe { self.storage.as_mut_ptr().add(off) as *mut u128 };
        let le = value.to_le();
        if (off & 15) == 0 {
            // SAFETY: `p` is in-bounds and properly aligned when `off` is 16-byte aligned.
            unsafe { p.write(le) };
        } else {
            unsafe { p.write_unaligned(le) };
        }
        Ok(())
    }

    #[inline(always)]
    fn write_bytes(&mut self, addr: usize, buf: &[u8]) -> Result<(), Box<Self::Fault>> {
        let r = self
            .checked_range_rel(AccessKind::Write, addr, buf.len())
            .map_err(Box::new)?;

        // SAFETY:
        // - `checked_range_rel` guarantees `[r.start, r.start + buf.len())` is in-bounds.
        // - `buf` is a valid readable slice of length `buf.len()`.
        // - Source and destination do not overlap (storage and `buf` are distinct allocations).
        unsafe {
            core::ptr::copy_nonoverlapping(
                buf.as_ptr(),
                self.storage.as_mut_ptr().add(r.start),
                buf.len(),
            );
        }
        Ok(())
    }
}
