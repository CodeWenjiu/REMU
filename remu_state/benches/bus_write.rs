use criterion::{Criterion, black_box, criterion_group, criterion_main};

use pprof::criterion::{Output, PProfProfiler};
use remu_state::bus::Bus;
use remu_types::Rv32;

mod common;

/// Write benchmark name (random full working set, aligned).
const BENCH_WRITE_NAME: &str = "bus_write_mixed_1_1_1_u8_u16_u32_aligned";

/// Variants to help distinguish "memory-bound" vs "overhead-bound" behavior.
///
/// - `*_sequential_*`: sequential access with good locality (prefetch-friendly)
/// - `*_small_ws_*`  : random access but small working set that fits in cache
const BENCH_WRITE_NAME_SEQUENTIAL: &str = "bus_write_mixed_1_1_1_u8_u16_u32_sequential";
const BENCH_WRITE_NAME_SMALL_WS: &str = "bus_write_mixed_1_1_1_u8_u16_u32_small_ws";

#[inline(never)]
fn run_write_workload(
    bus: &mut Bus<Rv32<true>>,
    addrs8: &[usize],
    addrs16: &[usize],
    addrs32: &[usize],
) {
    // Keep writes "observable" (avoid being optimized away):
    // write values derived from address and read them back.
    let mut acc: u64 = 0;

    for &addr in addrs8 {
        let v = (addr as u8).wrapping_mul(3);
        bus.write_8(addr, v)
            .expect("unmapped write_8 in bench workload");
        let r = bus.read_8(addr).expect("unmapped read_8 after write") as u64;
        acc = acc.wrapping_add(r);
    }
    for &addr in addrs16 {
        let v = (addr as u16).wrapping_mul(17);
        bus.write_16(addr, v)
            .expect("unmapped write_16 in bench workload");
        let r = bus.read_16(addr).expect("unmapped read_16 after write") as u64;
        acc = acc.wrapping_add(r);
    }
    for &addr in addrs32 {
        let v = (addr as u32).wrapping_mul(257);
        bus.write_32(addr, v)
            .expect("unmapped write_32 in bench workload");
        let r = bus.read_32(addr).expect("unmapped read_32 after write") as u64;
        acc = acc.wrapping_add(r);
    }

    black_box(acc);
}

fn bench_write(c: &mut Criterion) {
    let mut state = common::make_state_from_clap_defaults("bus_write_bench");

    let (addrs8, addrs16, addrs32) = common::prepare_workload_random_full();
    let (seq8, seq16, seq32) = common::prepare_workload_sequential();
    let (sw8, sw16, sw32) = common::prepare_workload_small_ws();

    c.bench_function(BENCH_WRITE_NAME, |b| {
        b.iter(|| run_write_workload(&mut state.bus, &addrs8, &addrs16, &addrs32))
    });

    c.bench_function(BENCH_WRITE_NAME_SEQUENTIAL, |b| {
        b.iter(|| run_write_workload(&mut state.bus, &seq8, &seq16, &seq32))
    });

    c.bench_function(BENCH_WRITE_NAME_SMALL_WS, |b| {
        b.iter(|| run_write_workload(&mut state.bus, &sw8, &sw16, &sw32))
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = bench_write
);
criterion_main!(benches);
