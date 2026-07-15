use core::fmt::Write;

/// A benchmark with prepare/run/validate phases.
pub(crate) trait Bench {
    /// Run this benchmark and return whether it passed.
    /// The writer can be used to emit diagnostic output (e.g. checksum on failure).
    fn run<W: Write>(w: &mut W) -> bool;
}

/// FNV-1a checksum with finalization, matching AM-Kernels microbench.
pub(crate) fn checksum(start: *const u8, end: *const u8) -> u32 {
    let x: u32 = 16777619;
    let mut h: u32 = 2166136261;
    let mut p = start;
    unsafe {
        while (p as usize + 4) < (end as usize) {
            for i in 0..4 {
                h = (h ^ (*p.add(i) as u32)).wrapping_mul(x);
            }
            p = p.add(4);
        }
    }
    let mut hash = h as i32;
    hash = hash.wrapping_add(hash << 13);
    hash ^= hash >> 7;
    hash = hash.wrapping_add(hash << 3);
    hash ^= hash >> 17;
    hash = hash.wrapping_add(hash << 5);
    hash as u32
}
