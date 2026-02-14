//! Vector register file (v0–v31). Type is `[[u8; VLENB]; 32]` when V is enabled, `()` when disabled.

use crate::AllUsize;

use super::RegDiff;

/// Vector register file state. When V is disabled use `()`; when enabled use `[[u8; VLENB]; 32]`.
pub trait VrState: Default + Clone + std::fmt::Debug + RegDiff {
    /// VLEN/8 in bytes; 0 when no V.
    const VLENB: u32;

    /// Read register `idx` (0..32) as slice of VLENB bytes.
    fn raw_read(&self, idx: usize) -> &[u8];

    /// Write register `idx` from `data` (len must be VLENB).
    fn raw_write(&mut self, idx: usize, data: &[u8]);
}

impl VrState for () {
    const VLENB: u32 = 0;

    #[inline(always)]
    fn raw_read(&self, _: usize) -> &[u8] {
        &[]
    }

    #[inline(always)]
    fn raw_write(&mut self, _: usize, _: &[u8]) {}
}

impl<const VLENB: usize> VrState for [[u8; VLENB]; 32]
where
    [u8; VLENB]: Default,
{
    const VLENB: u32 = VLENB as u32;

    #[inline(always)]
    fn raw_read(&self, idx: usize) -> &[u8] {
        &self[idx]
    }

    #[inline(always)]
    fn raw_write(&mut self, idx: usize, data: &[u8]) {
        self[idx].copy_from_slice(data);
    }
}

impl<const VLENB: usize> RegDiff for [[u8; VLENB]; 32]
where
    [u8; VLENB]: Default,
{
    fn diff(ref_this: &Self, dut: &Self) -> Vec<(String, AllUsize, AllUsize)> {
        (0..32)
            .filter_map(|i| {
                let (r, d) = (ref_this.raw_read(i), dut.raw_read(i));
                if r != d {
                    let (rv, dv) = if VLENB <= 16 {
                        let mut rb = [0u8; 16];
                        let mut db = [0u8; 16];
                        rb[..r.len()].copy_from_slice(r);
                        db[..d.len()].copy_from_slice(d);
                        (
                            AllUsize::U128(u128::from_le_bytes(rb)),
                            AllUsize::U128(u128::from_le_bytes(db)),
                        )
                    } else {
                        (AllUsize::U64(0), AllUsize::U64(0))
                    };
                    Some((format!("v{i}"), rv, dv))
                } else {
                    None
                }
            })
            .collect()
    }
}
