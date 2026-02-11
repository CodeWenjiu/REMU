use criterion::{Criterion, black_box, criterion_group, criterion_main};
use pprof::criterion::{Output, PProfProfiler};

use rand::{
    Rng,
    distr::{Distribution, weighted::WeightedIndex},
    rngs::ThreadRng,
};
use remu_simulator_remu::riscv::inst::{decode, RV32_INSTRUCTION_MIX};
use remu_state::StateFastProfile;
use remu_types::isa::extension_enum::RV32I;

/// Benchmark names used in `c.bench_function(...)`.
///
/// Criterion uses these to create output directories under `target/criterion/`.
/// When you enable Criterion-style profiling (see below), the flamegraph will be written to:
///
/// `target/criterion/<BENCH_NAME>/profile/flamegraph.svg`
const BENCH_NAME: &str = "decode_inst_stream";

struct InstructionGenerator {
    dist: WeightedIndex<u32>,
    opcodes: Vec<u32>,
}

impl InstructionGenerator {
    fn new() -> Self {
        let opcodes: Vec<u32> = RV32_INSTRUCTION_MIX.iter().map(|(op, _)| *op).collect();
        let weights: Vec<u32> = RV32_INSTRUCTION_MIX.iter().map(|(_, w)| *w).collect();

        let dist = WeightedIndex::new(&weights).expect("Failed to create weighted index");

        Self { dist, opcodes }
    }

    #[inline(always)]
    fn next(&self, rng: &mut ThreadRng) -> u32 {
        let idx = self.dist.sample(rng);
        let opcode = self.opcodes[idx];

        let random_bits = rng.random::<u32>();

        (random_bits & 0xFFFF_FF80) | opcode
    }
}

fn build_inst_stream(len: usize) -> Vec<u32> {
    let rng = &mut ThreadRng::default();
    let inst_gen = InstructionGenerator::new();
    (0..len).map(|_| inst_gen.next(rng)).collect()
}

#[inline(never)]
fn run_decode_workload(insts: &[u32]) {
    let mut acc: u64 = 0;
    for &inst in insts {
        let decoded = decode::<StateFastProfile<RV32I>>(inst);
        acc = acc.wrapping_add(decoded.imm as u64);
    }
    black_box(acc);
}

fn bench_decode(c: &mut Criterion) {
    // Pre-generate instructions to keep the hot loop focused on decode.
    let insts = build_inst_stream(1024);
    c.bench_function(BENCH_NAME, |b| b.iter(|| run_decode_workload(&insts)));
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
//     cargo bench -p remu_simulator_remu --bench decode
// - Flamegraph (for hotspots; profiling mode, analysis disabled):
//     cargo bench -p remu_simulator_remu --bench decode -- --profile-time 20
criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = bench_decode
);
criterion_main!(benches);
