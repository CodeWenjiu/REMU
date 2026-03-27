//! MNIST inference backends: [`MnistInference`] implemented by
//! [`WeightedInference`] (embedded INT8 weights) and [`Cus0Inference`] (CUS0 custom ISA only).

remu_macro::mod_flat!(cus0, cus0_asm, weighted);

use remu_hal::Vec;

pub use cus0::Cus0Inference;
pub use weighted::WeightedInference;

include!(concat!(env!("OUT_DIR"), "/embedded_images.rs"));

pub const Q16_SHIFT: u32 = 16;

pub(crate) const BENCHMARK_ITERATIONS: usize = 100;
pub(crate) const WARMUP_ITERATIONS: usize = 10;
pub(crate) const DETAILED_BENCHMARK_ITERATIONS: usize = 100;

/// Shared MNIST entry: run [`MnistInference::infer`] on embedded test images.
pub trait MnistInference {
    /// Classify a 784-byte raw image (UINT8 pixels).
    fn infer(&self, input_image: &[u8]) -> usize;

    /// Accuracy run over [`EMBEDDED_TEST_IMAGES`].
    fn test(&self) {
        let test_images_data = EMBEDDED_TEST_IMAGES;
        let total_images = test_images_data.len();
        let mut correct_predictions = 0;

        for (img_idx, image_data_bytes) in test_images_data.iter().enumerate() {
            remu_hal::println!("=== Test Image {} ===", img_idx + 1);

            let (image_data, true_label) = parse_image_binary(*image_data_bytes);
            remu_hal::println!("True label: {}", true_label);

            let predicted_digit = self.infer(&image_data);
            remu_hal::println!("Predicted:  {}", predicted_digit);

            if predicted_digit == true_label as usize {
                remu_hal::println!("✓ CORRECT PREDICTION!");
                correct_predictions += 1;
            } else {
                remu_hal::println!("❌ WRONG PREDICTION!");
            }
            remu_hal::println!();
        }

        remu_hal::println!("=== FINAL RESULTS ===");
        remu_hal::println!("Total images: {}", total_images);
        remu_hal::println!("Correct predictions: {}", correct_predictions);
        remu_hal::println!(
            "Accuracy: {:.2}%",
            (correct_predictions as f32 / total_images as f32) * 100.0
        );
    }

    /// Warmup + timed inference using the first embedded image.
    fn run_benchmark(&self) {
        remu_hal::println!("=== BENCHMARK MODE ===");
        remu_hal::println!("Warmup iterations: {}", WARMUP_ITERATIONS);
        remu_hal::println!("Benchmark iterations: {}", BENCHMARK_ITERATIONS);

        let benchmark_image_data = EMBEDDED_TEST_IMAGES[0];
        let (image_data, _) = parse_image_binary(benchmark_image_data);

        remu_hal::println!("Running warmup...");
        for _ in 0..WARMUP_ITERATIONS {
            let _ = self.infer(&image_data);
        }

        remu_hal::println!(
            "Running benchmark (CLINT mtime @ {} Hz)...",
            remu_hal::MTIME_TICK_HZ
        );

        let start_ticks = remu_hal::read_mtime();
        for _ in 0..BENCHMARK_ITERATIONS {
            let _ = self.infer(&image_data);
        }
        let end_ticks = remu_hal::read_mtime();
        let total_ticks = end_ticks.wrapping_sub(start_ticks);

        let ticks_per_inference = total_ticks / BENCHMARK_ITERATIONS as u64;
        let inferences_per_second = if total_ticks > 0 {
            (BENCHMARK_ITERATIONS as u128 * remu_hal::MTIME_TICK_HZ as u128 / total_ticks as u128)
                as u64
        } else {
            0
        };

        remu_hal::println!("=== BENCHMARK RESULTS ===");
        remu_hal::println!("Total mtime ticks: {}", total_ticks);
        remu_hal::println!("Iterations completed: {}", BENCHMARK_ITERATIONS);
        remu_hal::println!("Ticks per inference: {}", ticks_per_inference);
        remu_hal::println!(
            "Inferences per second (from mtime, {} Hz): {}",
            remu_hal::MTIME_TICK_HZ,
            inferences_per_second
        );

        remu_hal::println!("Performance classification:");
        if ticks_per_inference < 10_000 {
            remu_hal::println!("Excellent performance");
        } else if ticks_per_inference < 50_000 {
            remu_hal::println!("Good performance");
        } else if ticks_per_inference < 200_000 {
            remu_hal::println!("Moderate performance");
        } else {
            remu_hal::println!("Needs optimization");
        }

        let total_mac_operations =
            BENCHMARK_ITERATIONS as u64 * ((784 * 256) + (256 * 128) + (128 * 10)) as u64;
        let macs_per_tick = if total_ticks > 0 {
            total_mac_operations as f64 / total_ticks as f64
        } else {
            0.0
        };
        remu_hal::println!("Total MAC operations: {}", total_mac_operations);
        remu_hal::println!("MACs per mtime tick: {:.4}", macs_per_tick);
        remu_hal::println!("Note: Higher MACs/tick indicates better throughput at fixed mtime rate");
        if BENCHMARK_ITERATIONS > 0 {
            remu_hal::println!("Benchmark completed successfully");
        }
        remu_hal::println!("Use this as baseline for optimization comparisons");
    }

    /// Per-layer micro-benchmarks; default is a no-op (e.g. CUS0 has no CPU weights).
    fn detailed_performance_analysis(&self) {
        remu_hal::println!("=== DETAILED PERFORMANCE ANALYSIS ===");
        remu_hal::println!("(not implemented for this backend)");
    }
}

pub(crate) fn parse_image_binary(data: &[u8]) -> (Vec<u8>, u8) {
    let true_label = data[8];
    let image_data: Vec<u8> = data[9..].to_vec();
    (image_data, true_label)
}

/// Normalize UINT8 pixels to INT8 (same as previous [`WeightedInference`] path).
pub(crate) fn normalize_and_quantize_input(input: &[u8]) -> Vec<i8> {
    input
        .iter()
        .map(|&pixel| {
            let normalized = (pixel as f32) / 255.0;
            let quantized = (normalized * 127.0) as i32;
            if quantized < -128 {
                -128
            } else if quantized > 127 {
                127
            } else {
                quantized as i8
            }
        })
        .collect()
}
