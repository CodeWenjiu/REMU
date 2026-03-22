use remu_hal::{println, read_mtime, Vec, MTIME_TICK_HZ};

include!(concat!(env!("OUT_DIR"), "/embedded_images.rs"));

// Benchmark configuration
const BENCHMARK_ITERATIONS: usize = 100;
const WARMUP_ITERATIONS: usize = 10;
const DETAILED_BENCHMARK_ITERATIONS: usize = 100;
pub(crate) struct Inference {
    fc1_weights: [[i8; 784]; 256],
    fc2_weights: [[i8; 256]; 128],
    fc3_weights: [[i8; 128]; 10],
    fc1_scale_q16: i32,
    fc2_scale_q16: i32,
    fc3_scale_q16: i32,
}

// Compile-time weight parsing using const generics
const fn parse_weight_binary_const<const ROWS: usize, const COLS: usize>(
    data: &'static [u8],
) -> ([[i8; COLS]; ROWS], f32) {
    // Read scale at compile time
    let scale = f32::from_le_bytes([data[8], data[9], data[10], data[11]]);

    let mut weights = [[0i8; COLS]; ROWS];
    let mut i = 0;
    while i < ROWS {
        let mut j = 0;
        let start = 12 + i * COLS;
        while j < COLS {
            weights[i][j] = data[start + j] as i8;
            j += 1;
        }
        i += 1;
    }

    (weights, scale)
}

pub const Q16_SHIFT: u32 = 16;

/// Macro to convert float scale to Q16 fixed-point
macro_rules! scale_to_q16 {
    ($scale:expr) => {
        (($scale * (1 << Q16_SHIFT) as f32) as i32)
    };
}

impl Inference {
    pub(crate) fn new() -> Self {
        const FC1_WEIGHT_DATA: &[u8] = include_bytes!("../binarys/fc1_weight.bin");
        const FC2_WEIGHT_DATA: &[u8] = include_bytes!("../binarys/fc2_weight.bin");
        const FC3_WEIGHT_DATA: &[u8] = include_bytes!("../binarys/fc3_weight.bin");

        // Parse weights at compile time
        const FC1_PARSED: ([[i8; 784]; 256], f32) =
            parse_weight_binary_const::<256, 784>(FC1_WEIGHT_DATA);
        const FC2_PARSED: ([[i8; 256]; 128], f32) =
            parse_weight_binary_const::<128, 256>(FC2_WEIGHT_DATA);
        const FC3_PARSED: ([[i8; 128]; 10], f32) =
            parse_weight_binary_const::<10, 128>(FC3_WEIGHT_DATA);

        let (fc1_weights, fc1_scale) = FC1_PARSED;
        let (fc2_weights, fc2_scale) = FC2_PARSED;
        let (fc3_weights, fc3_scale) = FC3_PARSED;

        println!("Model weights loaded successfully!");
        println!(
            "FC1: {}x{}, scale: {:.6}",
            fc1_weights.len(),
            fc1_weights[0].len(),
            fc1_scale
        );
        println!(
            "FC2: {}x{}, scale: {:.6}",
            fc2_weights.len(),
            fc2_weights[0].len(),
            fc2_scale
        );
        println!(
            "FC3: {}x{}, scale: {:.6}\n",
            fc3_weights.len(),
            fc3_weights[0].len(),
            fc3_scale
        );

        let fc1_scale_q16 = scale_to_q16!(fc1_scale);
        let fc2_scale_q16 = scale_to_q16!(fc2_scale);
        let fc3_scale_q16 = scale_to_q16!(fc3_scale);

        // Print quantization information
        println!("Quantization Scales (Fixed Point):");
        println!("  FC1_SCALE: {:.6} -> Q16: {}", fc1_scale, fc1_scale_q16);
        println!("  FC2_SCALE: {:.6} -> Q16: {}", fc2_scale, fc2_scale_q16);
        println!("  FC3_SCALE: {:.6} -> Q16: {}", fc3_scale, fc3_scale_q16);
        println!();

        Self {
            fc1_weights,
            fc1_scale_q16,
            fc2_weights,
            fc2_scale_q16,
            fc3_weights,
            fc3_scale_q16,
        }
    }

    /// True pure INT8 MNIST inference (no floating point operations)
    ///
    /// Complete inference pipeline using only integer arithmetic:
    /// 1. Input normalization (UINT8 → INT8)
    /// 2. FC1 layer with ReLU6 activation
    /// 3. FC2 layer with ReLU6 activation
    /// 4. FC3 layer (output)
    /// 5. Classification (argmax)
    pub(crate) fn mnist_inference_pure_int8(&self, input_image: &[u8]) -> usize {
        // Step 1: Normalize input from UINT8 to INT8 [-128, 127] range
        let normalized_input = Self::normalize_and_quantize_input(input_image);

        // Step 2: Layer 1 - fc1
        let fc1_output = Self::int8_matmul_symmetric::<256, 784>(
            &self.fc1_weights,
            &normalized_input,
            self.fc1_scale_q16,
        );

        // Convert to INT8 for activation
        let mut fc1_activations = Self::int32_to_int8_with_scaling(&fc1_output);

        // Apply ReLU6
        Self::relu_int8(&mut fc1_activations);

        // Step 3: Layer 2 - fc2
        let fc2_output = Self::int8_matmul_symmetric::<128, 256>(
            &self.fc2_weights,
            &fc1_activations,
            self.fc2_scale_q16,
        );

        // Convert to INT8 for activation
        let mut fc2_activations = Self::int32_to_int8_with_scaling(&fc2_output);

        // Apply ReLU6
        Self::relu_int8(&mut fc2_activations);

        // Step 4: Layer 3 - fc3 (output layer)
        let final_output = Self::int8_matmul_symmetric::<10, 128>(
            &self.fc3_weights,
            &fc2_activations,
            self.fc3_scale_q16,
        );

        // Find predicted digit (argmax)
        Self::argmax_int32(&final_output)
    }

    pub(crate) fn test(&self) {
        let test_images_data = EMBEDDED_TEST_IMAGES;

        let total_images = test_images_data.len();
        let mut correct_predictions = 0;

        for (img_idx, image_data_bytes) in test_images_data.iter().enumerate() {
            println!("=== Test Image {} ===", img_idx + 1);

            let (image_data, true_label) = Self::parse_image_binary(*image_data_bytes);
            println!("True label: {}", true_label);

            // Run pure INT8 inference with embedded weights
            let predicted_digit = self.mnist_inference_pure_int8(&image_data);

            println!("Predicted:  {}", predicted_digit);

            if predicted_digit == true_label as usize {
                println!("✓ CORRECT PREDICTION!");
                correct_predictions += 1;
            } else {
                println!("❌ WRONG PREDICTION!");
            }

            println!();
        }

        // Summary
        println!("=== FINAL RESULTS ===");
        println!("Total images: {}", total_images);
        println!("Correct predictions: {}", correct_predictions);
        println!(
            "Accuracy: {:.2}%",
            (correct_predictions as f32 / total_images as f32) * 100.0
        );
    }

    pub(crate) fn run_benchmark(&self) {
        println!("=== BENCHMARK MODE ===");
        println!("Warmup iterations: {}", WARMUP_ITERATIONS);
        println!("Benchmark iterations: {}", BENCHMARK_ITERATIONS);

        // Use a representative test image for benchmarking
        let benchmark_image_data = EMBEDDED_TEST_IMAGES[0];
        let (image_data, _) = Self::parse_image_binary(benchmark_image_data);

        // Warmup phase
        println!("Running warmup...");
        for _ in 0..WARMUP_ITERATIONS {
            let _ = self.mnist_inference_pure_int8(&image_data);
        }

        // Benchmark phase: CLINT mtime @ 10 MHz (see remu_hal::MTIME_TICK_HZ)
        println!("Running benchmark (CLINT mtime @ {} Hz)...", MTIME_TICK_HZ);

        let start_ticks = read_mtime();

        for _ in 0..BENCHMARK_ITERATIONS {
            let _ = self.mnist_inference_pure_int8(&image_data);
        }

        let end_ticks = read_mtime();
        let total_ticks = end_ticks.wrapping_sub(start_ticks);

        // Calculate metrics (mtime ticks at MTIME_TICK_HZ)
        let ticks_per_inference = total_ticks / BENCHMARK_ITERATIONS as u64;
        let inferences_per_second = if total_ticks > 0 {
            (BENCHMARK_ITERATIONS as u128 * MTIME_TICK_HZ as u128 / total_ticks as u128) as u64
        } else {
            0
        };

        println!("=== BENCHMARK RESULTS ===");
        println!("Total mtime ticks: {}", total_ticks);
        println!("Iterations completed: {}", BENCHMARK_ITERATIONS);
        println!("Ticks per inference: {}", ticks_per_inference);
        println!(
            "Inferences per second (from mtime, {} Hz): {}",
            MTIME_TICK_HZ, inferences_per_second
        );

        // Performance classification (10 MHz → 10_000 ticks ≈ 1 ms / inference)
        println!("Performance classification:");
        if ticks_per_inference < 10_000 {
            println!("Excellent performance");
        } else if ticks_per_inference < 50_000 {
            println!("Good performance");
        } else if ticks_per_inference < 200_000 {
            println!("Moderate performance");
        } else {
            println!("Needs optimization");
        }

        // Performance analysis
        let total_mac_operations =
            BENCHMARK_ITERATIONS as u64 * ((784 * 256) + (256 * 128) + (128 * 10)) as u64;
        let macs_per_tick = if total_ticks > 0 {
            total_mac_operations as f64 / total_ticks as f64
        } else {
            0.0
        };

        println!("Total MAC operations: {}", total_mac_operations);
        println!("MACs per mtime tick: {:.4}", macs_per_tick);
        println!("Note: Higher MACs/tick indicates better throughput at fixed mtime rate");

        if BENCHMARK_ITERATIONS > 0 {
            println!("Benchmark completed successfully");
        }

        // Save baseline for comparison
        println!("Use this as baseline for optimization comparisons");
    }

    pub(crate) fn detailed_performance_analysis(&self) {
        println!("=== DETAILED PERFORMANCE ANALYSIS ===");

        let benchmark_image_data = EMBEDDED_TEST_IMAGES[0];
        let (image_data, _) = Self::parse_image_binary(benchmark_image_data);
        let normalized_input = Self::normalize_and_quantize_input(&image_data);

        // Benchmark individual components (CLINT mtime ticks)
        let mut total_ticks_sum: u64 = 0;

        // Benchmark normalize_input_pure_int8
        let start = read_mtime();
        for _ in 0..DETAILED_BENCHMARK_ITERATIONS {
            let _ = Self::normalize_and_quantize_input(&image_data);
        }
        let end = read_mtime();
        let norm_ticks = end.wrapping_sub(start) / DETAILED_BENCHMARK_ITERATIONS as u64;
        println!("normalize_input_pure_int8: {} mtime ticks/call", norm_ticks);
        total_ticks_sum += norm_ticks;

        // Benchmark FC1 matrix multiplication
        let start = read_mtime();
        for _ in 0..DETAILED_BENCHMARK_ITERATIONS {
            let _ = Self::int8_matmul_symmetric::<256, 784>(
                &self.fc1_weights,
                &normalized_input,
                self.fc1_scale_q16,
            );
        }
        let end = read_mtime();
        let fc1_ticks = end.wrapping_sub(start) / DETAILED_BENCHMARK_ITERATIONS as u64;
        println!("FC1 matmul (256x784): {} mtime ticks/call", fc1_ticks);
        total_ticks_sum += fc1_ticks;

        // Benchmark int32_to_int8_with_scaling
        let fc1_output = Self::int8_matmul_symmetric::<256, 784>(
            &self.fc1_weights,
            &normalized_input,
            self.fc1_scale_q16,
        );
        let start = read_mtime();
        for _ in 0..DETAILED_BENCHMARK_ITERATIONS {
            let _ = Self::int32_to_int8_with_scaling(&fc1_output);
        }
        let end = read_mtime();
        let scale_ticks = end.wrapping_sub(start) / DETAILED_BENCHMARK_ITERATIONS as u64;
        println!("int32_to_int8_with_scaling: {} mtime ticks/call", scale_ticks);
        total_ticks_sum += scale_ticks;

        // Benchmark relu6_int8
        let mut fc1_activations = Self::int32_to_int8_with_scaling(&fc1_output);
        let start = read_mtime();
        for _ in 0..DETAILED_BENCHMARK_ITERATIONS {
            Self::relu_int8(&mut fc1_activations);
        }
        let end = read_mtime();
        let relu_ticks = end.wrapping_sub(start) / DETAILED_BENCHMARK_ITERATIONS as u64;
        println!("relu6_int8: {} mtime ticks/call", relu_ticks);
        total_ticks_sum += relu_ticks;

        println!(
            "Estimated total mtime ticks (partial micro-bench sum): {}",
            total_ticks_sum
        );
        println!("Breakdown:");
        println!(
            "  - Input normalization: {:.1}%",
            (norm_ticks * 100) as f64 / total_ticks_sum as f64
        );
        println!(
            "  - FC1 matmul: {:.1}%",
            (fc1_ticks * 100) as f64 / total_ticks_sum as f64
        );
        println!(
            "  - Scaling: {:.1}%",
            (scale_ticks * 100) as f64 / total_ticks_sum as f64
        );
        println!(
            "  - Activation: {:.1}%",
            (relu_ticks * 100) as f64 / total_ticks_sum as f64
        );
    }

    /// Normalize input from UINT8 [0,255] to INT8 [-128,127] range
    ///
    /// Normalization formula: normalized = (pixel/255 * 2) - 1
    /// Fixed-point implementation: output = (input * 257 - 32768) >> 8
    fn normalize_and_quantize_input(input: &[u8]) -> Vec<i8> {
        input
            .iter()
            .map(|&pixel| {
                // Convert [0, 255] to [0, 1]
                let normalized = (pixel as f32) / 255.0;

                // Quantize symmetrically to INT8 range [-127, 127]
                // For [0, 1] range, scale to [-127, 127] is:
                // quantized = normalized * 127.0
                let quantized = (normalized * 127.0) as i32;

                // Clamp to INT8 range
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

    /// Pure INT8 matrix multiplication with symmetric scaling
    ///
    /// Operation: output = (weights * input) * scale
    /// All operations in integer arithmetic
    fn int8_matmul_symmetric<const ROWS: usize, const COLS: usize>(
        weights: &[[i8; COLS]; ROWS],
        input: &[i8],
        scale_q16: i32,
    ) -> Vec<i32> {
        let mut output = Vec::with_capacity(ROWS);

        // Simple nested loops - let LLVM handle vectorization
        for i in 0..ROWS {
            let mut sum: i32 = 0;
            for j in 0..COLS {
                sum += weights[i][j] as i32 * input[j] as i32;
            }

            // Apply scaling: sum * scale (in Q16 format)
            let scaled = (sum as i64 * scale_q16 as i64) >> Q16_SHIFT;

            output.push(scaled as i32);
        }

        output
    }

    /// ReLU activation for INT8 (clamp to [0, max_value])
    ///
    /// After ReLU, we keep positive values and clamp negatives to 0.
    /// The max value should be based on actual calibration data.
    /// Using a reasonable max to avoid overflow.
    fn relu_int8(data: &mut [i8]) {
        for val in data.iter_mut() {
            if *val < 0 {
                *val = 0;
            }
        }
    }

    /// Simple argmax for classification
    fn argmax_int32(data: &[i32]) -> usize {
        data.iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.cmp(b))
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    /// Convert INT32 to INT8 with dynamic scaling
    ///
    /// Automatically determines the optimal shift to preserve
    /// as much precision as possible while fitting into INT8 range
    fn int32_to_int8_with_scaling(input: &[i32]) -> Vec<i8> {
        // Find the maximum absolute value to determine scaling
        let max_abs = input.iter().fold(0, |acc, &x| acc.max(x.abs()));

        // If all values are zero, return zeros
        if max_abs == 0 {
            return vec![0; input.len()];
        }

        // Calculate shift needed to fit into INT8 range [-127, 127]
        let mut shift = 0;
        let mut max_val = max_abs;
        while max_val > 127 && shift < 31 {
            max_val >>= 1;
            shift += 1;
        }

        // Simple loop - let LLVM handle vectorization
        let mut result = Vec::with_capacity(input.len());
        for &x in input {
            result.push((x >> shift).clamp(-128, 127) as i8);
        }
        result
    }

    fn parse_image_binary(data: &[u8]) -> (Vec<u8>, u8) {
        // Read true label
        let true_label = data[8];

        // Read image data
        let image_data: Vec<u8> = data[9..].to_vec();

        (image_data, true_label)
    }
}