use criterion::{Criterion, criterion_group, criterion_main};

use clap::Parser;
use remu_state::bus::BusAccess;
use remu_state::{State, StateOption};
use remu_types::{AllUsize, DynDiagError, Tracer, TracerDyn};

struct BenchTracer;

impl Tracer for BenchTracer {
    #[inline(always)]
    fn mem_print(&self, _begin: usize, _data: &[u8], _result: Result<(), Box<dyn DynDiagError>>) {}

    #[inline(always)]
    fn mem_show(&self, _begin: usize, _data: Result<AllUsize, Box<dyn DynDiagError>>) {}

    #[inline(always)]
    fn deal_error(&self, _error: Box<dyn DynDiagError>) {}
}

fn bench_read(c: &mut Criterion) {
    // Build StateOption (and thus BusOption) via clap defaults by parsing an empty argv.
    // This picks up `default_value = "ram@0x8000_0000:0x0800_0000"` for `--mem`.
    #[derive(clap::Parser, Debug)]
    struct Opt {
        #[command(flatten)]
        state: StateOption,
    }

    let opt = Opt::parse_from(["memory_access_bench"]);
    let tracer: TracerDyn = std::rc::Rc::new(std::cell::RefCell::new(BenchTracer));
    let mut state = State::new(opt.state, tracer);

    const BASE: usize = 0x8000_0000;
    const SIZE: usize = 0x0800_0000;

    // Deterministic, fast PRNG so generated addresses are stable and we don't pull in extra deps.
    // (xorshift64*)
    let mut rng: u64 = 0x9E37_79B9_7F4A_7C15;

    // Exact 1:1:1 ratio, but stored separately to minimize per-access overhead.
    // Total reads per benchmark iteration = 3 * PER_KIND.
    const PER_KIND: usize = 1 << 15; // 32768 each -> 98304 total reads/iter

    let mut addrs8: Vec<usize> = Vec::with_capacity(PER_KIND);
    let mut addrs16: Vec<usize> = Vec::with_capacity(PER_KIND);
    let mut addrs32: Vec<usize> = Vec::with_capacity(PER_KIND);

    for _ in 0..PER_KIND {
        // read_8 addr
        rng ^= rng >> 12;
        rng ^= rng << 25;
        rng ^= rng >> 27;
        let x8 = rng.wrapping_mul(0x2545_F491_4F6C_DD1D);
        addrs8.push(BASE + ((x8 >> 2) as usize % SIZE));

        // read_16 aligned addr
        rng ^= rng >> 12;
        rng ^= rng << 25;
        rng ^= rng >> 27;
        let x16 = rng.wrapping_mul(0x2545_F491_4F6C_DD1D);
        let max16 = SIZE - 2;
        let off16 = ((x16 >> 2) as usize % (max16 + 1)) & !1usize;
        addrs16.push(BASE + off16);

        // read_32 aligned addr
        rng ^= rng >> 12;
        rng ^= rng << 25;
        rng ^= rng >> 27;
        let x32 = rng.wrapping_mul(0x2545_F491_4F6C_DD1D);
        let max32 = SIZE - 4;
        let off32 = ((x32 >> 2) as usize % (max32 + 1)) & !3usize;
        addrs32.push(BASE + off32);
    }

    c.bench_function("bus_read_mixed_1_1_1_u8_u16_u32_aligned", |b| {
        b.iter(|| {
            // 这三段循环可以显著减少每次访存的额外开销（避免 match 分支），
            // 同时仍然保持总调用比例严格为 1:1:1。
            for &addr in &addrs8 {
                let _ = state
                    .bus
                    .read_8(addr)
                    .expect("unmapped read_8 in bench workload");
            }
            for &addr in &addrs16 {
                let _ = state
                    .bus
                    .read_16(addr)
                    .expect("unmapped read_16 in bench workload");
            }
            for &addr in &addrs32 {
                let _ = state
                    .bus
                    .read_32(addr)
                    .expect("unmapped read_32 in bench workload");
            }
        })
    });
}

criterion_group!(benches, bench_read);
criterion_main!(benches);
