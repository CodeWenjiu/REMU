#![no_std]
#![no_main]

#[macro_use]
extern crate alloc;

remu_macro::mod_pub!(inference);
use remu_hal::{entry, FmtWrite, Uart16550, exit_success};

use crate::inference::{MnistInference};

/// Switch backend: [`WeightedInference`] (CPU + weights) or [`crate::inference::Cus0Inference`].
type Engine = crate::inference::WeightedInference;

static BENCHMARK_MODE: bool = false;

#[entry]
fn main() -> ! {
    // `init()` 内含 `pre_main_init()`，并初始化全局堆；分配前必须调用。
    unsafe { remu_hal::init() };
    let mut uart = Uart16550::default_base();

    let infer = Engine::new();

    // Run benchmarks based on mode
    if BENCHMARK_MODE {
        // Benchmark-only mode - skip accuracy testing
        let _ = writeln!(uart, "=== BENCHMARK-ONLY MODE ===");

        infer.detailed_performance_analysis();
        let _ = writeln!(uart);

        // Run full inference benchmark
        infer.run_benchmark();

        exit_success()
    } else {
        // Normal mode - run quick benchmark then accuracy test
        let _ = writeln!(uart, "=== QUICK BENCHMARK ===");
        infer.run_benchmark();
        let _ = writeln!(uart);
    }

    infer.test();
    exit_success()
}
