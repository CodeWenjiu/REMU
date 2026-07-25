#![cfg_attr(target_arch = "riscv32", no_std, no_main)]

extern crate alloc;

mod bench;
mod benches;

use bench::{Bench, Size};
use benches::{bf, dinic, fib, lzip, md5, pz15, qsort, queen, sieve, ssort};

fn get_size() -> Size {
    #[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
    {
        let arg = std::env::args().nth(1).unwrap_or_default();
        Size::from_arg(&arg)
    }
    #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
    {
        Size::from_arg(remu_hal::app_args())
    }
}

#[cfg_attr(target_arch = "riscv32", remu_hal::entry)]
fn main() -> ! {
    remu_hal::init();
    let size = get_size();
    remu_hal::println!("=== microbench [{}] ===", size.name());

    let mut total_score = 0u64;
    let mut n_scored = 0u64;
    let t0 = bench::now_usec();

    run::<queen::Queen>("queen", size, &mut total_score, &mut n_scored);
    run::<qsort::Qsort>("qsort", size, &mut total_score, &mut n_scored);
    run::<sieve::Sieve>("sieve", size, &mut total_score, &mut n_scored);
    run::<bf::Bf>("bf", size, &mut total_score, &mut n_scored);
    run::<fib::Fib>("fib", size, &mut total_score, &mut n_scored);
    run::<md5::Md5>("md5", size, &mut total_score, &mut n_scored);
    run::<dinic::Dinic>("dinic", size, &mut total_score, &mut n_scored);
    run::<ssort::Ssort>("ssort", size, &mut total_score, &mut n_scored);
    run::<pz15::Pz15>("15pz", size, &mut total_score, &mut n_scored);
    run::<lzip::Lzip>("lzip", size, &mut total_score, &mut n_scored);

    let total_time = bench::now_usec() - t0;
    if n_scored > 0 {
        let avg = total_score / n_scored;
        remu_hal::println!("Score: {} (vs {} on Core Ultra 5 125H)", avg, 100000u64);
    }
    remu_hal::println!(
        "Total time: {}.{:03} ms",
        total_time / 1000,
        total_time % 1000
    );

    remu_hal::exit_success()
}

fn run<B: Bench>(name: &str, size: Size, total_score: &mut u64, n_scored: &mut u64) {
    let mut out = remu_hal::Uart16550::default_base();

    let t0 = bench::now_usec();
    let passed = B::run(&mut out, size);
    let usec = bench::now_usec() - t0;

    let ref_time = B::ref_time_usec(size);
    let sc = if ref_time > 0 {
        bench::score(ref_time, usec)
    } else {
        0
    };

    let status = if passed { "PASS" } else { "FAIL" };
    if sc > 0 {
        remu_hal::println!(
            "{}: {} {:>5}.{:03} ms [{}]",
            name,
            status,
            usec / 1000,
            usec % 1000,
            sc
        );
        *total_score += sc;
        *n_scored += 1;
    } else {
        remu_hal::println!(
            "{}: {} {:>5}.{:03} ms",
            name,
            status,
            usec / 1000,
            usec % 1000
        );
    }
}
