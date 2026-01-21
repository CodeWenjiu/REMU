use criterion::{Criterion, black_box, criterion_group, criterion_main};

use clap::Parser;
use pprof::criterion::{Output, PProfProfiler};
use remu_state::bus::{BusAccess, BusOption};
use remu_state::{State, StateOption};
use remu_types::{AllUsize, DynDiagError, Tracer, TracerDyn};

/// Benchmark name used in `c.bench_function(...)`.
///
/// Criterion uses this to create output directories under `target/criterion/`.
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

/// A minimal tracer implementation so we can construct `remu_state::State` without pulling in CLI.
///
/// This benchmark focuses on bus/memory access; tracer output would only add noise.
/// We still use `State` because `Bus::new` is `pub(crate)` and benches are compiled as separate crates.
struct BenchTracer;

impl Tracer for BenchTracer {
    #[inline(always)]
    fn mem_print(&self, _begin: usize, _data: &[u8], _result: Result<(), Box<dyn DynDiagError>>) {}

    #[inline(always)]
    fn mem_show(&self, _begin: usize, _data: Result<AllUsize, Box<dyn DynDiagError>>) {}

    #[inline(always)]
    fn deal_error(&self, _error: Box<dyn DynDiagError>) {}
}

#[inline(never)]
fn make_state_from_clap_defaults() -> State {
    // ----------------------------
    // IMPORTANT: Constructing State/Bus in a bench
    // ----------------------------
    //
    // `BusOption` is defined with clap attributes, including a `default_value` for `--mem`.
    // The "default bus" maps a RAM region at:
    //
    //   [0x8000_0000, 0x8800_0000)  (base 0x8000_0000, size 0x0800_0000)
    //
    // We want benches to be representative of the real binary's configuration, so we
    // intentionally *reuse the clap defaults*, instead of repeating the memory mapping logic
    // in the bench itself.
    //
    // NOTE: `Bus::new` is `pub(crate)` inside `remu_state`, and benchmarks are separate crates
    // under `benches/`, so we can't call `Bus::new` directly. We therefore construct a `State`
    // (which is public) and access `state.bus`.
    //
    // Build StateOption (and thus BusOption) via clap defaults by parsing an empty argv.
    #[derive(clap::Parser, Debug)]
    struct Opt {
        #[command(flatten)]
        state: StateOption,
    }

    // `parse_from(["..."])` triggers clap's default_value filling behavior.
    let opt = Opt::parse_from(["memory_access_bench"]);

    // Sanity check: ensure the clap default actually populated BusOption.mem, so our address
    // range is mapped. If someone changes the default later, this bench will fail loudly
    // instead of producing misleading "unmapped"/error-heavy profiles.
    let BusOption { mem } = &opt.state.bus;
    assert!(
        !mem.is_empty(),
        "BusOption.mem is empty; clap default for --mem did not apply"
    );

    // BenchTracer does nothing; it exists only because `State::new` requires a tracer.
    let tracer: TracerDyn = std::rc::Rc::new(std::cell::RefCell::new(BenchTracer));
    State::new(opt.state, tracer)
}

#[inline(never)]
fn prepare_workload() -> (Vec<usize>, Vec<usize>, Vec<usize>) {
    // ----------------------------
    // Workload generation (reproducible + aligned + inside mapped RAM)
    // ----------------------------
    //
    // We generate addresses *once* outside the measured loop:
    // - avoids measuring RNG/addr-gen overhead
    // - makes measurements more stable/reproducible
    //
    // Address constraints (we intentionally avoid unaligned accesses for now):
    // - read_8 : any addr in range
    // - read_16: addr % 2 == 0
    // - read_32: addr % 4 == 0
    //
    // All addresses must fall in the mapped RAM region created by the default BusOption:
    //   [BASE, BASE + SIZE)
    const BASE: usize = 0x8000_0000;
    const SIZE: usize = 0x0800_0000;

    // Deterministic, fast PRNG so generated addresses are stable across runs without extra deps.
    // (xorshift64*)
    let mut rng: u64 = 0x9E37_79B9_7F4A_7C15;

    // We keep a strict 1:1:1 ratio. Using separate arrays removes a per-access match/branch,
    // which keeps the benchmark focused on the memory access implementation.
    //
    // Total reads per benchmark iteration = 3 * PER_KIND.
    const PER_KIND: usize = 1 << 15; // 32768 each -> 98304 total reads/iter

    let mut addrs8: Vec<usize> = Vec::with_capacity(PER_KIND);
    let mut addrs16: Vec<usize> = Vec::with_capacity(PER_KIND);
    let mut addrs32: Vec<usize> = Vec::with_capacity(PER_KIND);

    for _ in 0..PER_KIND {
        // read_8 addr: [BASE, BASE+SIZE)
        rng ^= rng >> 12;
        rng ^= rng << 25;
        rng ^= rng >> 27;
        let x8 = rng.wrapping_mul(0x2545_F491_4F6C_DD1D);
        addrs8.push(BASE + ((x8 >> 2) as usize % SIZE));

        // read_16 aligned addr: [BASE, BASE+SIZE-2], even
        rng ^= rng >> 12;
        rng ^= rng << 25;
        rng ^= rng >> 27;
        let x16 = rng.wrapping_mul(0x2545_F491_4F6C_DD1D);
        let max16 = SIZE - 2;
        let off16 = ((x16 >> 2) as usize % (max16 + 1)) & !1usize;
        addrs16.push(BASE + off16);

        // read_32 aligned addr: [BASE, BASE+SIZE-4], 4-byte aligned
        rng ^= rng >> 12;
        rng ^= rng << 25;
        rng ^= rng >> 27;
        let x32 = rng.wrapping_mul(0x2545_F491_4F6C_DD1D);
        let max32 = SIZE - 4;
        let off32 = ((x32 >> 2) as usize % (max32 + 1)) & !3usize;
        addrs32.push(BASE + off32);
    }

    (addrs8, addrs16, addrs32)
}

#[inline(never)]
fn prepare_workload_sequential() -> (Vec<usize>, Vec<usize>, Vec<usize>) {
    // Sequential variant:
    // - good spatial locality
    // - friendly to HW prefetchers
    // - helps answer: are we memory-latency bound (random) or overhead bound?
    const BASE: usize = 0x8000_0000;
    const SIZE: usize = 0x0800_0000;

    const PER_KIND: usize = 1 << 15;

    let mut addrs8: Vec<usize> = Vec::with_capacity(PER_KIND);
    let mut addrs16: Vec<usize> = Vec::with_capacity(PER_KIND);
    let mut addrs32: Vec<usize> = Vec::with_capacity(PER_KIND);

    // Keep within bounds and preserve alignment requirements.
    // Interleave different strides so this doesn't accidentally become "tiny working set".
    for i in 0..PER_KIND {
        let off8 = i % SIZE;
        addrs8.push(BASE + off8);

        let max16 = SIZE - 2;
        let off16 = ((2 * i) % (max16 + 1)) & !1usize;
        addrs16.push(BASE + off16);

        let max32 = SIZE - 4;
        let off32 = ((4 * i) % (max32 + 1)) & !3usize;
        addrs32.push(BASE + off32);
    }

    (addrs8, addrs16, addrs32)
}

#[inline(never)]
fn prepare_workload_small_ws() -> (Vec<usize>, Vec<usize>, Vec<usize>) {
    // Small working set random variant:
    // - random accesses but confined to a small region likely to fit in cache
    // - helps separate "cache-miss bound" from "instruction/overhead bound"
    const BASE: usize = 0x8000_0000;

    // Keep this small enough to typically fit in LLC (and often in L2/L3 depending on CPU).
    // Tune later if needed.
    const SIZE: usize = 256 * 1024;

    let mut rng: u64 = 0xD1B5_4A32_D192_ED03;

    const PER_KIND: usize = 1 << 15;

    let mut addrs8: Vec<usize> = Vec::with_capacity(PER_KIND);
    let mut addrs16: Vec<usize> = Vec::with_capacity(PER_KIND);
    let mut addrs32: Vec<usize> = Vec::with_capacity(PER_KIND);

    for _ in 0..PER_KIND {
        rng ^= rng >> 12;
        rng ^= rng << 25;
        rng ^= rng >> 27;
        let x8 = rng.wrapping_mul(0x2545_F491_4F6C_DD1D);
        addrs8.push(BASE + ((x8 >> 2) as usize % SIZE));

        rng ^= rng >> 12;
        rng ^= rng << 25;
        rng ^= rng >> 27;
        let x16 = rng.wrapping_mul(0x2545_F491_4F6C_DD1D);
        let max16 = SIZE - 2;
        let off16 = ((x16 >> 2) as usize % (max16 + 1)) & !1usize;
        addrs16.push(BASE + off16);

        rng ^= rng >> 12;
        rng ^= rng << 25;
        rng ^= rng >> 27;
        let x32 = rng.wrapping_mul(0x2545_F491_4F6C_DD1D);
        let max32 = SIZE - 4;
        let off32 = ((x32 >> 2) as usize % (max32 + 1)) & !3usize;
        addrs32.push(BASE + off32);
    }

    (addrs8, addrs16, addrs32)
}

#[inline(never)]
fn run_workload(
    bus: &mut impl BusAccess<Fault = remu_state::bus::MemFault>,
    addrs8: &[usize],
    addrs16: &[usize],
    addrs32: &[usize],
) {
    // ----------------------------
    // The measured loop body
    // ----------------------------
    //
    // Keep the hot loop in a named function so flamegraphs show a stable "entry point"
    // (instead of just `{{closure}}` frames).
    //
    // IMPORTANT: Always "use" the loaded value.
    // If you read a value and then discard it, LLVM may be able to remove or shrink the load,
    // resulting in a benchmark that over-emphasizes address mapping logic (e.g. region lookup)
    // and under-emphasizes actual data reads.
    //
    // We accumulate into `acc` and then feed it to `criterion::black_box` so the compiler cannot
    // assume the loads are unused.
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
    // Setup is intentionally outside the timed loop so we measure read performance, not init.
    let mut state = make_state_from_clap_defaults();

    // Baseline: random access across the full mapped RAM region.
    let (addrs8, addrs16, addrs32) = prepare_workload();

    // Variant: sequential access (prefetch-friendly).
    let (seq8, seq16, seq32) = prepare_workload_sequential();

    // Variant: random access but small working set (more cache-resident).
    let (sw8, sw16, sw32) = prepare_workload_small_ws();

    // Criterion will run these closures many times to collect statistics.
    // Keep per-iteration overhead minimal and deterministic.
    c.bench_function(BENCH_NAME, |b| {
        b.iter(|| run_workload(&mut state.bus, &addrs8, &addrs16, &addrs32))
    });

    c.bench_function(BENCH_NAME_SEQUENTIAL, |b| {
        b.iter(|| run_workload(&mut state.bus, &seq8, &seq16, &seq32))
    });

    c.bench_function(BENCH_NAME_SMALL_WS, |b| {
        b.iter(|| run_workload(&mut state.bus, &sw8, &sw16, &sw32))
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
//     cargo bench -p remu_state --bench memory_access
// - Flamegraph (for hotspots; profiling mode, analysis disabled):
//     cargo bench -p remu_state --bench memory_access -- --profile-time 20
criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = bench_read
);
criterion_main!(benches);
