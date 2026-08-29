use core::fmt::Write;

/// Reference score for the baseline CPU (i9-9900K).
const REF_SCORE: u64 = 100_000;

/// Benchmark data size, matching AM-Kernels microbench tiers.
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Size {
    Test,
    Train,
    Ref,
    Huge,
}

impl Size {
    /// Parse a size from a CLI argument. Returns `None` for unknown input.
    pub(crate) fn from_arg(arg: &str) -> Option<Self> {
        match arg {
            "test" => Some(Size::Test),
            "train" => Some(Size::Train),
            "ref" => Some(Size::Ref),
            "huge" => Some(Size::Huge),
            _ => None,
        }
    }

    /// All accepted CLI argument names, space-separated (for error messages).
    pub(crate) fn accepted_args() -> &'static str {
        "test, train, ref, huge"
    }

    pub(crate) fn name(&self) -> &str {
        match self {
            Size::Test => "test",
            Size::Train => "train",
            Size::Ref => "ref",
            Size::Huge => "huge",
        }
    }
}

/// A benchmark with name, reference time, run, and scoring.
pub(crate) trait Bench {
    /// Reference execution time in microseconds on REF_CPU (i9-9900K).
    /// Returns 0 for sizes that don't support scoring (test, train).
    fn ref_time_usec(size: Size) -> u64;
    /// Run this benchmark and return whether it passed.
    fn run<W: Write>(w: &mut W, size: Size) -> bool;
}

/// Calculate score: REF_SCORE * ref_time / actual_time_usec.
pub(crate) fn score(ref_time_usec: u64, actual_usec: u64) -> u64 {
    if actual_usec == 0 {
        0
    } else {
        REF_SCORE * ref_time_usec / actual_usec
    }
}

/// Platform-independent clock read. Returns microseconds elapsed since an arbitrary epoch.
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
pub(crate) fn now_usec() -> u64 {
    remu_hal::read_mtime() / 10
}

#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
pub(crate) fn now_usec() -> u64 {
    static START: std::sync::LazyLock<std::time::Instant> =
        std::sync::LazyLock::new(std::time::Instant::now);
    START.elapsed().as_micros() as u64
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
