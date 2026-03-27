//! INT8 MNIST with embedded FC weights and scales (CPU inference).

use remu_hal::{println, read_mtime, Vec};

use super::{
    normalize_and_quantize_input, parse_image_binary, DETAILED_BENCHMARK_ITERATIONS,
    EMBEDDED_TEST_IMAGES, MnistInference, Q16_SHIFT,
};

/// Loads `binarys/*.bin` at build time and runs the MLP on the CPU.
pub struct WeightedInference {
    fc1_weights: [[i8; 784]; 256],
    fc2_weights: [[i8; 256]; 128],
    fc3_weights: [[i8; 128]; 10],
    fc1_scale_q16: i32,
    fc2_scale_q16: i32,
    fc3_scale_q16: i32,
}

const fn parse_weight_binary_const<const ROWS: usize, const COLS: usize>(
    data: &'static [u8],
) -> ([[i8; COLS]; ROWS], f32) {
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

macro_rules! scale_to_q16 {
    ($scale:expr) => {
        (($scale * (1 << Q16_SHIFT) as f32) as i32)
    };
}

impl WeightedInference {
    pub fn new() -> Self {
        const FC1_WEIGHT_DATA: &[u8] = include_bytes!("../../binarys/fc1_weight.bin");
        const FC2_WEIGHT_DATA: &[u8] = include_bytes!("../../binarys/fc2_weight.bin");
        const FC3_WEIGHT_DATA: &[u8] = include_bytes!("../../binarys/fc3_weight.bin");

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

    fn mnist_inference_from_normalized(&self, normalized_input: &[i8]) -> usize {
        let fc1_output = Self::int8_matmul_symmetric::<256, 784>(
            &self.fc1_weights,
            normalized_input,
            self.fc1_scale_q16,
        );

        let mut fc1_activations = Self::int32_to_int8_with_scaling(&fc1_output);
        Self::relu_int8(&mut fc1_activations);

        let fc2_output = Self::int8_matmul_symmetric::<128, 256>(
            &self.fc2_weights,
            &fc1_activations,
            self.fc2_scale_q16,
        );

        let mut fc2_activations = Self::int32_to_int8_with_scaling(&fc2_output);
        Self::relu_int8(&mut fc2_activations);

        let final_output = Self::int8_matmul_symmetric::<10, 128>(
            &self.fc3_weights,
            &fc2_activations,
            self.fc3_scale_q16,
        );

        Self::argmax_int32(&final_output)
    }

    fn int8_matmul_symmetric<const ROWS: usize, const COLS: usize>(
        weights: &[[i8; COLS]; ROWS],
        input: &[i8],
        scale_q16: i32,
    ) -> Vec<i32> {
        let mut output = Vec::with_capacity(ROWS);
        for i in 0..ROWS {
            let mut sum: i32 = 0;
            for j in 0..COLS {
                sum += weights[i][j] as i32 * input[j] as i32;
            }
            let scaled = (sum as i64 * scale_q16 as i64) >> Q16_SHIFT;
            output.push(scaled as i32);
        }
        output
    }

    fn relu_int8(data: &mut [i8]) {
        for val in data.iter_mut() {
            if *val < 0 {
                *val = 0;
            }
        }
    }

    fn argmax_int32(data: &[i32]) -> usize {
        data.iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.cmp(b))
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    fn int32_to_int8_with_scaling(input: &[i32]) -> Vec<i8> {
        let max_abs = input.iter().fold(0, |acc, &x| acc.max(x.abs()));
        if max_abs == 0 {
            return vec![0; input.len()];
        }
        let mut shift = 0;
        let mut max_val = max_abs;
        while max_val > 127 && shift < 31 {
            max_val >>= 1;
            shift += 1;
        }
        let mut result = Vec::with_capacity(input.len());
        for &x in input {
            result.push((x >> shift).clamp(-128, 127) as i8);
        }
        result
    }
}

impl MnistInference for WeightedInference {
    fn infer(&self, input_image: &[u8]) -> usize {
        let normalized_input = normalize_and_quantize_input(input_image);
        self.mnist_inference_from_normalized(&normalized_input)
    }

    /// After [`MnistInference::test`] / [`MnistInference::run_benchmark`], runs layer micro-benchmarks.
    fn detailed_performance_analysis(&self) {
        println!("=== DETAILED PERFORMANCE ANALYSIS ===");

        let benchmark_image_data = EMBEDDED_TEST_IMAGES[0];
        let (image_data, _) = parse_image_binary(benchmark_image_data);
        let normalized_input = normalize_and_quantize_input(&image_data);

        let mut total_ticks_sum: u64 = 0;

        let start = read_mtime();
        for _ in 0..DETAILED_BENCHMARK_ITERATIONS {
            let _ = normalize_and_quantize_input(&image_data);
        }
        let end = read_mtime();
        let norm_ticks = end.wrapping_sub(start) / DETAILED_BENCHMARK_ITERATIONS as u64;
        println!("normalize_input_pure_int8: {} mtime ticks/call", norm_ticks);
        total_ticks_sum += norm_ticks;

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
}
