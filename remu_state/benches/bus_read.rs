use criterion::{Criterion, black_box, criterion_group, criterion_main};

use pprof::criterion::{Output, PProfProfiler};
use remu_state::bus::Bus;
use remu_types::Rv32I;

mod common;

/// Benchmark names used in `c.bench_function(...)`.
///
/// Criterion uses these to create output directories under `target/criterion/`.
/// When you enable Criterion-style profiling (see below), the flamegraph will be written to:
///
/// `target/criterion/<BENCH_NAME>/profile/flamegraph.svg`
const BENCH_NAME: &str = "bus_read_mixed_1_1_1_u8_u16_u32_aligned";

/// Variants to help distinguish "memory-bound" vs "overhead-bound" behavior.
///
/// - `*_sequential_*`: sequential access with good locality (prefetch-friendly)
/// - `*_small_ws_*`  : random access but small working set that fits in cache
const BENCH_NAME_SEQUENTIAL: &str = "bus_read_mixed_1_1_1_u8_u16_u32_sequential";
const BENCH_NAME_SMALL_WS: &str = "bus_read_mixed_1_1_1_u8_u16_u32_small_ws";

#[inline(never)]
fn run_read_workload(bus: &mut Bus<Rv32I>, addrs8: &[usize], addrs16: &[usize], addrs32: &[usize]) {
    let mut acc: u64 = 0;

    for &addr in addrs8 {
        let v = bus.read_8(addr).expect("unmapped read_8 in bench workload") as u64;
        acc = acc.wrapping_add(v);
    }
    for &addr in addrs16 {
        let v = bus
            .read_16(addr)
            .expect("unmapped read_16 in bench workload") as u64;
        acc = acc.wrapping_add(v);
    }
    for &addr in addrs32 {
        let v = bus
            .read_32(addr)
            .expect("unmapped read_32 in bench workload") as u64;
        acc = acc.wrapping_add(v);
    }

    black_box(acc);
}

fn bench_read(c: &mut Criterion) {
    // Construct state (allocates RAM backing storage).
    let mut state = common::make_state_from_clap_defaults("bus_read_bench");

    let (addrs8, addrs16, addrs32) = common::prepare_workload_random_full();
    let (seq8, seq16, seq32) = common::prepare_workload_sequential();
    let (sw8, sw16, sw32) = common::prepare_workload_small_ws();

    c.bench_function(BENCH_NAME, |b| {
        b.iter(|| run_read_workload(&mut state.bus, &addrs8, &addrs16, &addrs32))
    });

    c.bench_function(BENCH_NAME_SEQUENTIAL, |b| {
        b.iter(|| run_read_workload(&mut state.bus, &seq8, &seq16, &seq32))
    });

    c.bench_function(BENCH_NAME_SMALL_WS, |b| {
        b.iter(|| run_read_workload(&mut state.bus, &sw8, &sw16, &sw32))
    });
}

// ----------------------------
// Flamegraph / profiling mode
// ----------------------------
//
// We integrate pprof via Criterion's `Profiler` hook. This has a few practical implications:
//
// 1) When running with `-- --profile-time <secs>`, Criterion enters "profiling mode":
//    - It profiles for the requested duration and outputs `flamegraph.svg` under
//      `target/criterion/<BENCH_NAME>/profile/`.
//    - Criterion typically disables statistical comparison output in profiling mode.
//      That's expected: profiling perturbs timing and would otherwise pollute baselines.
//    - Use profiling runs for "where is the time going?", not for "did it regress?".
//
// 2) When running without `-- --profile-time ...`, you get normal benchmark statistics
//    (baselines/new/change/report). Use those runs to compare before/after performance.
//
// 3) Profiling quality depends on build configuration:
//    - For readable flamegraphs, you generally want symbols enabled (debuginfo) and avoid stripping.
//    - In this repo, we use a bench profile that keeps symbols while keeping release-like optimizations.
//      (See workspace `Cargo.toml` `[profile.bench]`.)
//    - If you change inlining/LTO/strip settings, the flamegraph call stacks can become shallower.
//
// Quick commands:
// - Normal benchmark (for comparison):
//     cargo bench -p remu_state --bench bus_read
// - Flamegraph (for hotspots; profiling mode, analysis disabled):
//     cargo bench -p remu_state --bench bus_read -- --profile-time 20
criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = bench_read
);
criterion_main!(benches);
