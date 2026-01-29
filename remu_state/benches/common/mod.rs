use std::ops::Range;

use clap::Parser;

use remu_state::bus::BusOption;
use remu_state::{State, StateOption};
use remu_types::isa::reg::Gpr;
use remu_types::{AllUsize, DynDiagError, Tracer, TracerDyn, isa::extension_enum::RV32I};

/// A minimal tracer implementation so we can construct `remu_state::State` without pulling in CLI.
///
/// This benchmark focuses on bus/memory access; tracer output would only add noise.
pub struct BenchTracer;

impl Tracer for BenchTracer {
    #[inline(always)]
    fn print(&self, _message: &str) {}

    #[inline(always)]
    fn mem_print(&self, _begin: usize, _data: &[u8], _result: Result<(), Box<dyn DynDiagError>>) {}

    #[inline(always)]
    fn mem_show(&self, _begin: usize, _data: Result<AllUsize, Box<dyn DynDiagError>>) {}

    #[inline(always)]
    fn mem_show_map(&self, _map: Vec<(String, Range<usize>)>) {}

    #[inline(always)]
    fn reg_show(&self, _index: Gpr, _data: u32) {}

    #[inline(always)]
    fn reg_print(&self, _regs: &[(Gpr, u32); 32], _range: Range<usize>) {}

    #[inline(always)]
    fn disasm(&self, _pc: u64, _inst: u32) {}
}

#[inline(never)]
pub fn make_state_from_clap_defaults(tag: &'static str) -> State<RV32I> {
    // We intentionally reuse clap defaults so benches match real configuration.
    #[derive(clap::Parser, Debug)]
    struct Opt {
        #[command(flatten)]
        state: StateOption,
    }

    let opt = Opt::parse_from([tag]);

    // Sanity check: ensure the clap default actually populated BusOption.mem, so our address
    // range is mapped. If someone changes the default later, this bench will fail loudly
    // instead of producing misleading "unmapped"/error-heavy profiles.
    let BusOption { mem, elf: _ } = &opt.state.bus;
    assert!(
        !mem.is_empty(),
        "BusOption.mem is empty; clap default for --mem did not apply"
    );

    let tracer: TracerDyn = std::rc::Rc::new(std::cell::RefCell::new(BenchTracer));
    State::new(opt.state, tracer)
}

#[inline(never)]
pub fn prepare_workload_random_full() -> (Vec<usize>, Vec<usize>, Vec<usize>) {
    // Default mapped RAM region:
    //   [0x8000_0000, 0x8800_0000)
    const BASE: usize = 0x8000_0000;
    const SIZE: usize = 0x0800_0000;

    // Deterministic PRNG (xorshift64*)
    let mut rng: u64 = 0x9E37_79B9_7F4A_7C15;

    // Strict 1:1:1:1 distribution.
    const PER_KIND: usize = 1 << 15; // 32768 each -> 131072 total ops/iter

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
pub fn prepare_workload_sequential() -> (Vec<usize>, Vec<usize>, Vec<usize>) {
    const BASE: usize = 0x8000_0000;
    const SIZE: usize = 0x0800_0000;

    const PER_KIND: usize = 1 << 15;

    let mut addrs8: Vec<usize> = Vec::with_capacity(PER_KIND);
    let mut addrs16: Vec<usize> = Vec::with_capacity(PER_KIND);
    let mut addrs32: Vec<usize> = Vec::with_capacity(PER_KIND);

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
pub fn prepare_workload_small_ws() -> (Vec<usize>, Vec<usize>, Vec<usize>) {
    const BASE: usize = 0x8000_0000;
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
