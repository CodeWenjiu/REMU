//! Deadlock watchdog for the nzea backend: monitors architectural progress (commits)
//! from a background thread. When no commit is reported for `misses × interval`, the
//! thread prints what it saw, flags the trip, and raises the shared interrupt flag so
//! the run stops; the blocked simulator's poll then reports `Stalled` instead of the
//! Ctrl+C `Interrupted`.
//!
//! Format (the value of `--sim-opt watchdog=`):
//!   `5`     → 3 misses × 5s interval (default misses=3)
//!   `3x10`  → 3 misses × 10s interval
//!   `test`  → 1 miss × 0s: trips as soon as it is not fed (for tests)
//!   `off`   → disabled
//!
//! A core that keeps retiring instructions is alive; one that stops committing
//! (deadlock, or an access that never completes) stops feeding this watchdog.
//! Workloads that legitimately go long between commits should raise the interval or
//! miss count rather than disable it.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use winnow::Parser;
use winnow::ascii::float;
use winnow::combinator::{opt, terminated};
use winnow::token::take_while;

pub(crate) struct Watchdog {
    /// Cycle of the last commit; written by `feed`, drained by the thread.
    cycle: Arc<AtomicU64>,
    /// Set by the thread when it gives up on the run; read by `tripped`.
    tripped: Arc<AtomicBool>,
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
        let tripped = Arc::new(AtomicBool::new(false));
        let tripped_clone = Arc::clone(&tripped);

        let handle = thread::Builder::new()
            .name("nzea-watchdog".into())
            .spawn(move || {
                let mut consecutive = 0u32;
                let mut last_commit_cycle = 0u64;
                loop {
                    thread::sleep(interval);
                    let prev = cycle_clone.swap(0, Ordering::Relaxed);
                    if prev == 0 {
                        consecutive += 1;
                        if consecutive >= misses {
                            // Detection point: report what we saw, then let the run
                            // stop. The error that reaches the top carries no data.
                            eprintln!(
                                "nzea watchdog: no commit for {misses} × {interval:?} \
                                 (last commit at cycle {last_commit_cycle}); stopping the run"
                            );
                            tripped_clone.store(true, Ordering::Relaxed);
                            interrupt.store(true, Ordering::Relaxed);
                            break;
                        }
                    } else {
                        last_commit_cycle = prev;
                        consecutive = 0;
                    }
                }
            })
            .map_err(|e| format!("failed to spawn watchdog thread: {e}"))?;

        Ok(Some(Self {
            cycle,
            tripped,
            _handle: Some(handle),
        }))
    }

    /// Feed the watchdog: called for every committed instruction, with the sim cycle
    /// it committed at (kept so a trip can say where the last commit was).
    #[inline(always)]
    pub(crate) fn feed(&self, cycle_count: u64) {
        self.cycle.store(cycle_count, Ordering::Relaxed);
    }

    /// True once the thread gave up on the run; the poll then fails as `Stalled`.
    #[inline(always)]
    pub(crate) fn tripped(&self) -> bool {
        self.tripped.load(Ordering::Relaxed)
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
    use std::time::Instant;

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

    #[test]
    fn trips_when_nothing_is_fed() {
        let interrupt = Arc::new(AtomicBool::new(false));
        let watchdog = Watchdog::from_spec(Some("test"), Arc::clone(&interrupt)).unwrap();
        let watchdog = watchdog.expect("spec `test` should arm the watchdog");

        let deadline = Instant::now() + Duration::from_secs(10);
        while !watchdog.tripped() {
            assert!(Instant::now() < deadline, "watchdog never fired");
            thread::yield_now();
        }
        // A trip also raises the shared flag, so anything polling it stops the run.
        assert!(interrupt.load(Ordering::Relaxed));
    }
}
