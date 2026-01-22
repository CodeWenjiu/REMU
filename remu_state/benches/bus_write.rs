use criterion::{Criterion, black_box, criterion_group, criterion_main};

use pprof::criterion::{Output, PProfProfiler};

use remu_state::bus::BusAccess;

mod common;

/// Write benchmark name (random full working set, aligned).
const BENCH_WRITE_NAME: &str = "bus_write_mixed_1_1_1_u8_u16_u32_u64_aligned";

/// Variants to help distinguish "memory-bound" vs "overhead-bound" behavior.
///
/// - `*_sequential_*`: sequential access with good locality (prefetch-friendly)
/// - `*_small_ws_*`  : random access but small working set that fits in cache
const BENCH_WRITE_NAME_SEQUENTIAL: &str = "bus_write_mixed_1_1_1_u8_u16_u32_u64_sequential";
const BENCH_WRITE_NAME_SMALL_WS: &str = "bus_write_mixed_1_1_1_u8_u16_u32_u64_small_ws";

/// Fixed-size `write_bytes` benchmarks (cache-line-like).
///
/// We keep the length fixed to make the work per iteration stable and comparable.
/// These benches are expected to become more relevant once a cache-line writeback path exists.
const BENCH_WRITE_BYTES_NAME: &str = "bus_write_bytes_64B_aligned";
const BENCH_WRITE_BYTES_NAME_SEQUENTIAL: &str = "bus_write_bytes_64B_sequential";
const BENCH_WRITE_BYTES_NAME_SMALL_WS: &str = "bus_write_bytes_64B_small_ws";

#[inline(never)]
fn run_write_workload(
    bus: &mut impl BusAccess<Fault = remu_state::bus::BusFault>,
    addrs8: &[usize],
    addrs16: &[usize],
    addrs32: &[usize],
    addrs64: &[usize],
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
    for &addr in addrs64 {
        let v = (addr as u64).wrapping_mul(65_537);
        bus.write_64(addr, v)
            .expect("unmapped write_64 in bench workload");
        let r = bus.read_64(addr).expect("unmapped read_64 after write") as u64;
        acc = acc.wrapping_add(r);
    }

    black_box(acc);
}

#[inline(never)]
fn run_write_bytes_workload_64b(
    bus: &mut impl BusAccess<Fault = remu_state::bus::BusFault>,
    addrs64_aligned: &[usize],
) {
    // Fixed-size "cache line" writes.
    // Use a stack buffer to keep allocation out of the hot loop.
    let mut acc: u64 = 0;
    let mut buf = [0u8; 64];

    for &addr in addrs64_aligned {
        // Deterministic payload, changes with address to avoid trivially redundant stores.
        buf[0] = (addr as u8).wrapping_mul(3);
        buf[63] = (addr as u8).wrapping_mul(7);

        bus.write_bytes(addr, &buf)
            .expect("unmapped write_bytes(64) in bench workload");

        // Read back a couple bytes to keep side effects observable.
        let mut verify = [0u8; 64];
        bus.read_bytes(addr, &mut verify)
            .expect("unmapped read_bytes(64) after write_bytes");
        acc = acc.wrapping_add(verify[0] as u64);
        acc = acc.wrapping_add(verify[63] as u64);
    }

    black_box(acc);
}

fn bench_write(c: &mut Criterion) {
    let mut state = common::make_state_from_clap_defaults("bus_write_bench");

    let (addrs8, addrs16, addrs32, addrs64) = common::prepare_workload_random_full();
    let (seq8, seq16, seq32, seq64) = common::prepare_workload_sequential();
    let (sw8, sw16, sw32, sw64) = common::prepare_workload_small_ws();

    c.bench_function(BENCH_WRITE_NAME, |b| {
        b.iter(|| run_write_workload(&mut state.bus, &addrs8, &addrs16, &addrs32, &addrs64))
    });

    c.bench_function(BENCH_WRITE_NAME_SEQUENTIAL, |b| {
        b.iter(|| run_write_workload(&mut state.bus, &seq8, &seq16, &seq32, &seq64))
    });

    c.bench_function(BENCH_WRITE_NAME_SMALL_WS, |b| {
        b.iter(|| run_write_workload(&mut state.bus, &sw8, &sw16, &sw32, &sw64))
    });

    // write_bytes(64) benchmarks: reuse the 64-bit aligned address stream (8-byte aligned implies
    // also safe for 64B writes as long as the underlying mapping is large enough, which it is).
    c.bench_function(BENCH_WRITE_BYTES_NAME, |b| {
        b.iter(|| run_write_bytes_workload_64b(&mut state.bus, &addrs64))
    });

    c.bench_function(BENCH_WRITE_BYTES_NAME_SEQUENTIAL, |b| {
        b.iter(|| run_write_bytes_workload_64b(&mut state.bus, &seq64))
    });

    c.bench_function(BENCH_WRITE_BYTES_NAME_SMALL_WS, |b| {
        b.iter(|| run_write_bytes_workload_64b(&mut state.bus, &sw64))
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = bench_write
);
criterion_main!(benches);
