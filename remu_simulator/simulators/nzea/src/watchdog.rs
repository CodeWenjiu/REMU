//! Deadlock watchdog: monitors sim cycle progress via a background thread.
//! When the harness stops advancing cycles for `misses × interval`, the thread sets the
//! shared interrupt flag, causing `step_once` to return `Interrupted`.
//!
//! Format (parsed from `--sim-opt nzea.watchdog=SPEC`):
//!   `5s`      → 3 misses × 5s interval (default misses=3)
//!   `3x10s`   → 3 misses × 10s interval
//!   `off`     → disabled
//!
//! Harness feeds the watchdog every 1024 sim cycles by writing the current cycle count.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

pub(crate) struct Watchdog {
    /// Current sim cycle count; harness updates this every 1024 cycles.
    cycle: Arc<AtomicU64>,
    /// Cloned into the thread for the join handle.
    _handle: Option<thread::JoinHandle<()>>,
}

impl Watchdog {
    /// Parse `spec` and spawn a watchdog thread if enabled.
    /// Returns `None` when spec is `"off"` or empty.
    pub(crate) fn from_spec(spec: Option<&str>, interrupt: Arc<AtomicBool>) -> Option<Self> {
        let spec = spec?.trim();
        if spec.is_empty() || spec.eq_ignore_ascii_case("off") {
            return None;
        }
        let (misses, interval) = parse_spec(spec)?;

        let cycle = Arc::new(AtomicU64::new(0));
        let cycle_clone = Arc::clone(&cycle);

        let handle = thread::Builder::new()
            .name("nzea-watchdog".into())
            .spawn(move || {
                let mut consecutive = 0u32;
                loop {
                    thread::sleep(interval);
                    let prev = cycle_clone.swap(0, Ordering::Relaxed);
                    if prev == 0 {
                        consecutive += 1;
                        if consecutive >= misses {
                            interrupt.store(true, Ordering::Relaxed);
                            break;
                        }
                    } else {
                        consecutive = 0;
                    }
                }
            })
            .ok()?;

        Some(Self {
            cycle,
            _handle: Some(handle),
        })
    }

    /// Feed the watchdog: write the current sim cycle count (called every 1024 cycles).
    #[inline(always)]
    pub(crate) fn feed(&self, cycle_count: u64) {
        self.cycle.store(cycle_count, Ordering::Relaxed);
    }
}

/// Parse `"3x10s"`, `"5s"` into (misses, interval).
fn parse_spec(s: &str) -> Option<(u32, Duration)> {
    let s = s.trim();
    let (misses_str, dur_str) = if let Some((left, right)) = s.split_once('x') {
        (left.trim(), right.trim())
    } else {
        ("3", s)
    };
    let misses: u32 = misses_str.parse().ok()?;
    if misses == 0 {
        return None;
    }
    let secs: f64 = dur_str.strip_suffix('s')?.parse().ok()?;
    if secs <= 0.0 {
        return None;
    }
    let interval = Duration::from_secs_f64(secs);
    Some((misses, interval))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_default_misses() {
        let (m, d) = parse_spec("5s").unwrap();
        assert_eq!(m, 3);
        assert_eq!(d, Duration::from_secs(5));
    }

    #[test]
    fn parse_explicit() {
        let (m, d) = parse_spec("2x10s").unwrap();
        assert_eq!(m, 2);
        assert_eq!(d, Duration::from_secs(10));
    }

    #[test]
    fn parse_fractional() {
        let (m, d) = parse_spec("3x0.5s").unwrap();
        assert_eq!(m, 3);
        assert_eq!(d, Duration::from_millis(500));
    }
}
