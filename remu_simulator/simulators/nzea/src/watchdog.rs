//! Deadlock watchdog: monitors sim cycle progress via a background thread.
//! When the harness stops advancing cycles for `misses × interval`, the thread sets the
//! shared interrupt flag, causing `step_once` to return `Interrupted`.
//!
//! Format (parsed from `--sim-opt nzea.watchdog=SPEC`):
//!   `5`     → 3 misses × 5s interval (default misses=3)
//!   `3x10`  → 3 misses × 10s interval
//!   `off`   → disabled
//!
//! Harness feeds the watchdog every 1024 sim cycles by writing the current cycle count.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use winnow::Parser;
use winnow::ascii::float;
use winnow::combinator::{opt, terminated};
use winnow::token::take_while;

pub(crate) struct Watchdog {
    /// Current sim cycle count; harness updates this every 1024 cycles.
    cycle: Arc<AtomicU64>,
    /// Cloned into the thread for the join handle.
    _handle: Option<thread::JoinHandle<()>>,
}

impl Watchdog {
    /// Parse `spec` and spawn a watchdog thread if enabled.
    /// Returns `Ok(None)` when spec is None, empty, or "off".
    /// Returns an error for invalid specs.
    pub(crate) fn from_spec(
        spec: Option<&str>,
        interrupt: Arc<AtomicBool>,
    ) -> Result<Option<Self>, String> {
        let spec = spec.map(str::trim).unwrap_or("");
        if spec.is_empty() || spec.eq_ignore_ascii_case("off") {
            return Ok(None);
        }
        let (misses, interval) = if spec.eq_ignore_ascii_case("test") {
            (1, Duration::ZERO)
        } else {
            parse_spec(spec)
                .ok_or_else(|| format!("invalid --sim-opt watchdog={spec:?}: expected a number (e.g. 5), NxM (e.g. 3x10), test, or off"))?
        };

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
            .map_err(|e| format!("failed to spawn watchdog thread: {e}"))?;

        Ok(Some(Self {
            cycle,
            _handle: Some(handle),
        }))
    }

    /// Feed the watchdog: write the current sim cycle count (called every 1024 cycles).
    #[inline(always)]
    pub(crate) fn feed(&self, cycle_count: u64) {
        self.cycle.store(cycle_count, Ordering::Relaxed);
    }
}

/// Parse `"3x10"`, `"5"` into (misses, interval).
fn parse_spec(s: &str) -> Option<(u32, Duration)> {
    parse_spec_impl.parse(s.trim()).ok()
}

fn parse_spec_impl(input: &mut &str) -> winnow::Result<(u32, Duration)> {
    let misses: u32 = opt(terminated(
        take_while(1.., |c: char| c.is_ascii_digit()),
        'x',
    ))
    .map(|s: Option<&str>| s.unwrap_or("3").parse().unwrap())
    .parse_next(input)?;
    let secs: f64 = float.parse_next(input)?;
    if secs <= 0.0 || misses == 0 {
        return Err(winnow::error::ContextError::new());
    }
    Ok((misses, Duration::from_secs_f64(secs)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_default_misses() {
        let (m, d) = parse_spec("5").unwrap();
        assert_eq!(m, 3);
        assert_eq!(d, Duration::from_secs(5));
    }

    #[test]
    fn parse_explicit() {
        let (m, d) = parse_spec("2x10").unwrap();
        assert_eq!(m, 2);
        assert_eq!(d, Duration::from_secs(10));
    }

    #[test]
    fn parse_fractional() {
        let (m, d) = parse_spec("3x0.5").unwrap();
        assert_eq!(m, 3);
        assert_eq!(d, Duration::from_millis(500));
    }
}
