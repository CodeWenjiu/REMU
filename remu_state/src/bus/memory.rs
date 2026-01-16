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
    pub base: u64,
    pub size: u64,
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

        fn parse_u64_allow_hex_underscore(s: &str, field: &str) -> Result<u64, String> {
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
                u64::from_str_radix(hex, 16).map_err(|e| {
                    format!(
                        "invalid mem region spec: {} '{}' is not valid hex: {}",
                        field, raw, e
                    )
                })?
            } else if cleaned.chars().all(|c| c.is_ascii_digit()) {
                cleaned.parse::<u64>().map_err(|e| {
                    format!(
                        "invalid mem region spec: {} '{}' is not valid decimal: {}",
                        field, raw, e
                    )
                })?
            } else {
                u64::from_str_radix(&cleaned, 16).map_err(|e| {
                    format!(
                        "invalid mem region spec: {} '{}' is not valid hex: {}",
                        field, raw, e
                    )
                })?
            };

            Ok(value)
        }

        let base = parse_u64_allow_hex_underscore(base_str, "base")?;
        let size = parse_u64_allow_hex_underscore(size_str, "size")?;

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
    Unmapped { addr: u64 },

    #[error(
        "out of bounds {kind:?} at 0x{addr:016x} (size={size}) for region '{region}' \
         [0x{base:016x}..0x{end:016x})"
    )]
    OutOfBounds {
        kind: AccessKind,
        addr: u64,
        size: usize,
        region: String,
        base: u64,
        end: u64,
    },

    #[error(
        "misaligned {kind:?} at 0x{addr:016x} (alignment={align}) for region '{region}' \
         [0x{base:016x}..0x{end:016x})"
    )]
    Misaligned {
        kind: AccessKind,
        addr: u64,
        align: usize,
        region: String,
        base: u64,
        end: u64,
    },

    #[error("invalid region '{name}': size too large to allocate on this platform: {size}")]
    SizeTooLarge { name: String, size: u64 },

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
    pub base: u64,
    pub end: u64, // exclusive
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
    pub fn contains(&self, addr: u64) -> bool {
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
        rel_addr: u64,
        size: usize,
    ) -> Result<Range<usize>, MemFault> {
        // Fast reject on end overflow: rel_addr+size must be <= region_size
        let region_size = self.storage.len();

        let off = usize::try_from(rel_addr).map_err(|_| MemFault::OutOfBounds {
            kind,
            addr: self.base.wrapping_add(rel_addr),
            size,
            region: self.name.clone(),
            base: self.base,
            end: self.end,
        })?;

        let end = off.checked_add(size).ok_or_else(|| MemFault::OutOfBounds {
            kind,
            addr: self.base.wrapping_add(rel_addr),
            size,
            region: self.name.clone(),
            base: self.base,
            end: self.end,
        })?;

        if end > region_size {
            return Err(MemFault::OutOfBounds {
                kind,
                addr: self.base.wrapping_add(rel_addr),
                size,
                region: self.name.clone(),
                base: self.base,
                end: self.end,
            });
        }

        Ok(off..end)
    }

    #[inline(always)]
    fn check_aligned(&self, kind: AccessKind, addr: u64, align: usize) -> Result<(), MemFault> {
        if align <= 1 {
            return Ok(());
        }
        let mask = (align - 1) as u64;
        if (addr & mask) != 0 {
            return Err(MemFault::Misaligned {
                kind,
                addr,
                align,
                region: self.name.clone(),
                base: self.base,
                end: self.end,
            });
        }
        Ok(())
    }

    #[inline(always)]
    pub fn read8_rel(&mut self, rel_addr: u64) -> Result<u8, MemFault> {
        let r = self.checked_range_rel(AccessKind::Read, rel_addr, 1)?;
        Ok(self.storage[r.start])
    }

    #[inline(always)]
    pub fn write8_rel(&mut self, rel_addr: u64, value: u8) -> Result<(), MemFault> {
        let r = self.checked_range_rel(AccessKind::Write, rel_addr, 1)?;
        self.storage[r.start] = value;
        Ok(())
    }

    #[inline(always)]
    pub fn read16_rel(&mut self, rel_addr: u64) -> Result<u16, MemFault> {
        // alignment is identical for absolute and relative since base is aligned by mapping choice
        self.check_aligned(AccessKind::Read, self.base + rel_addr, 2)?;
        let r = self.checked_range_rel(AccessKind::Read, rel_addr, 2)?;
        let bytes = [self.storage[r.start], self.storage[r.start + 1]];
        Ok(u16::from_le_bytes(bytes))
    }

    #[inline(always)]
    pub fn write16_rel(&mut self, rel_addr: u64, value: u16) -> Result<(), MemFault> {
        self.check_aligned(AccessKind::Write, self.base + rel_addr, 2)?;
        let r = self.checked_range_rel(AccessKind::Write, rel_addr, 2)?;
        let bytes = value.to_le_bytes();
        self.storage[r.start] = bytes[0];
        self.storage[r.start + 1] = bytes[1];
        Ok(())
    }

    #[inline(always)]
    pub fn read32_rel(&mut self, rel_addr: u64) -> Result<u32, MemFault> {
        self.check_aligned(AccessKind::Read, self.base + rel_addr, 4)?;
        let r = self.checked_range_rel(AccessKind::Read, rel_addr, 4)?;
        let bytes = [
            self.storage[r.start],
            self.storage[r.start + 1],
            self.storage[r.start + 2],
            self.storage[r.start + 3],
        ];
        Ok(u32::from_le_bytes(bytes))
    }

    #[inline(always)]
    pub fn write32_rel(&mut self, rel_addr: u64, value: u32) -> Result<(), MemFault> {
        self.check_aligned(AccessKind::Write, self.base + rel_addr, 4)?;
        let r = self.checked_range_rel(AccessKind::Write, rel_addr, 4)?;
        let bytes = value.to_le_bytes();
        self.storage[r.start] = bytes[0];
        self.storage[r.start + 1] = bytes[1];
        self.storage[r.start + 2] = bytes[2];
        self.storage[r.start + 3] = bytes[3];
        Ok(())
    }

    #[inline(always)]
    pub fn read64_rel(&mut self, rel_addr: u64) -> Result<u64, MemFault> {
        self.check_aligned(AccessKind::Read, self.base + rel_addr, 8)?;
        let r = self.checked_range_rel(AccessKind::Read, rel_addr, 8)?;
        let bytes = [
            self.storage[r.start],
            self.storage[r.start + 1],
            self.storage[r.start + 2],
            self.storage[r.start + 3],
            self.storage[r.start + 4],
            self.storage[r.start + 5],
            self.storage[r.start + 6],
            self.storage[r.start + 7],
        ];
        Ok(u64::from_le_bytes(bytes))
    }

    #[inline(always)]
    pub fn write64_rel(&mut self, rel_addr: u64, value: u64) -> Result<(), MemFault> {
        self.check_aligned(AccessKind::Write, self.base + rel_addr, 8)?;
        let r = self.checked_range_rel(AccessKind::Write, rel_addr, 8)?;
        let bytes = value.to_le_bytes();
        self.storage[r.start] = bytes[0];
        self.storage[r.start + 1] = bytes[1];
        self.storage[r.start + 2] = bytes[2];
        self.storage[r.start + 3] = bytes[3];
        self.storage[r.start + 4] = bytes[4];
        self.storage[r.start + 5] = bytes[5];
        self.storage[r.start + 6] = bytes[6];
        self.storage[r.start + 7] = bytes[7];
        Ok(())
    }
}

impl BusAccess for Memory {
    type Fault = MemFault;

    #[inline(always)]
    fn read_8(&mut self, addr: u64) -> Result<u8, Self::Fault> {
        self.read8_rel(addr)
    }

    #[inline(always)]
    fn read_16(&mut self, addr: u64) -> Result<u16, Self::Fault> {
        self.read16_rel(addr)
    }

    #[inline(always)]
    fn read_32(&mut self, addr: u64) -> Result<u32, Self::Fault> {
        self.read32_rel(addr)
    }

    #[inline(always)]
    fn read_64(&mut self, addr: u64) -> Result<u64, Self::Fault> {
        self.read64_rel(addr)
    }

    #[inline(always)]
    fn write_8(&mut self, addr: u64, value: u8) -> Result<(), Self::Fault> {
        self.write8_rel(addr, value)
    }

    #[inline(always)]
    fn write_16(&mut self, addr: u64, value: u16) -> Result<(), Self::Fault> {
        self.write16_rel(addr, value)
    }

    #[inline(always)]
    fn write_32(&mut self, addr: u64, value: u32) -> Result<(), Self::Fault> {
        self.write32_rel(addr, value)
    }

    #[inline(always)]
    fn write_64(&mut self, addr: u64, value: u64) -> Result<(), Self::Fault> {
        self.write64_rel(addr, value)
    }
}
