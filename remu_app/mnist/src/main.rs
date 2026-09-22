#[macro_use]
extern crate alloc;

remu_macro::mod_pub!(crate, inference);
use remu_hal::entry;
use remu_hal::exit_success;

use crate::inference::MnistInference;

/// Switch backend: [`crate::inference::WeightedInference`] (CPU + weights) or [`crate::inference::Cus0Inference`].
type Engine = crate::inference::Cus0Inference;

static BENCHMARK_MODE: bool = false;

#[entry]
fn main() -> ! {
    // `init()` 内含 `pre_main_init()`，并初始化全局堆；分配前必须调用。
    remu_hal::init();

    let infer = Engine::new();

    // Run benchmarks based on mode
    if BENCHMARK_MODE {
        // Benchmark-only mode - skip accuracy testing
        remu_hal::println!("=== BENCHMARK-ONLY MODE ===");

        infer.detailed_performance_analysis();
        remu_hal::println!();

        // Run full inference benchmark
        infer.run_benchmark();

        exit_success()
    } else {
        // Normal mode - run quick benchmark then accuracy test
        remu_hal::println!("=== QUICK BENCHMARK ===");
        infer.run_benchmark();
        remu_hal::println!();
    }

    infer.test();
    exit_success()
}
