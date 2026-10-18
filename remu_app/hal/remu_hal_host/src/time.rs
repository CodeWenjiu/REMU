//! CLINT-equivalent time base for host runs.

/// MTIME tick frequency (host: 1000 ticks/sec — `read_mtime` returns ms).
pub const MTIME_TICK_HZ: u64 = 1000;

/// Read CLINT mtime (host: milliseconds since process start).
#[inline]
pub fn read_mtime() -> u64 {
    use std::sync::OnceLock;
    use std::time::Instant;
    // Monotonic ms since process start — good enough for animation timing.
    static START: OnceLock<Instant> = OnceLock::new();
    let _ = START.get_or_init(Instant::now);
    START.get().unwrap().elapsed().as_millis() as u64
}
