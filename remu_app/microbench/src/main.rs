#![cfg_attr(target_arch = "riscv32", no_std, no_main)]

extern crate alloc;

mod bench;
mod benches;

use bench::Bench;
use benches::*;

#[cfg_attr(target_arch = "riscv32", remu_hal::entry)]
fn main() -> ! {
    remu_hal::init();
    remu_hal::println!("=== microbench ===");

    run::<Queen>("queen");
    run::<Qsort>("qsort");
    run::<Sieve>("sieve");
    run::<Bf>("bf");
    run::<Fib>("fib");
    run::<Md5>("md5");
    run::<Dinic>("dinic");
    run::<Ssort>("ssort");
    run::<Pz15>("15pz");
    run::<Lzip>("lzip");

    remu_hal::exit_success()
}

fn run<B: Bench>(name: &str) {
    let mut out = remu_hal::Uart16550::default_base();
    let passed = B::run(&mut out);
    remu_hal::println!("{}: {}", name, if passed { "PASS" } else { "FAIL" });
}
