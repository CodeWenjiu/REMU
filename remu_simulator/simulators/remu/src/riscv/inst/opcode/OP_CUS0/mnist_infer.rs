//! Embedded INT8 MNIST MLP weights + forward (matches `remu_app/mnist` [`WeightedInference`] math).

use std::sync::{Mutex, OnceLock};

const Q16_SHIFT: u32 = 16;

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

const fn scale_to_q16(scale: f32) -> i32 {
    (scale * ((1u32 << Q16_SHIFT) as f32)) as i32
}

const FC1_RAW: &[u8] = include_bytes!("fc1_weight.bin");
const FC2_RAW: &[u8] = include_bytes!("fc2_weight.bin");
const FC3_RAW: &[u8] = include_bytes!("fc3_weight.bin");

const FC1_PARSED: ([[i8; 784]; 256], f32) = parse_weight_binary_const::<256, 784>(FC1_RAW);
const FC2_PARSED: ([[i8; 256]; 128], f32) = parse_weight_binary_const::<128, 256>(FC2_RAW);
const FC3_PARSED: ([[i8; 128]; 10], f32) = parse_weight_binary_const::<10, 128>(FC3_RAW);

/// Weights + scales (same layout as mnist `WeightedInference`).
pub(crate) struct MnistWeights {
    pub fc1_weights: [[i8; 784]; 256],
    pub fc2_weights: [[i8; 256]; 128],
    pub fc3_weights: [[i8; 128]; 10],
    pub fc1_scale_q16: i32,
    pub fc2_scale_q16: i32,
    pub fc3_scale_q16: i32,
}

pub(crate) const WEIGHTS: MnistWeights = MnistWeights {
    fc1_weights: FC1_PARSED.0,
    fc2_weights: FC2_PARSED.0,
    fc3_weights: FC3_PARSED.0,
    fc1_scale_q16: scale_to_q16(FC1_PARSED.1),
    fc2_scale_q16: scale_to_q16(FC2_PARSED.1),
    fc3_scale_q16: scale_to_q16(FC3_PARSED.1),
};

/// Buffered input and computed logits (simulated accelerator).
pub(crate) struct Cus0AccelState {
    pub input: [i8; 784],
    pub logits: [i32; 10],
}

impl Default for Cus0AccelState {
    fn default() -> Self {
        Self {
            input: [0i8; 784],
            logits: [0i32; 10],
        }
    }
}

static ACCEL: OnceLock<Mutex<Cus0AccelState>> = OnceLock::new();

fn accel() -> &'static Mutex<Cus0AccelState> {
    ACCEL.get_or_init(|| Mutex::new(Cus0AccelState::default()))
}

#[inline]
pub(crate) fn buffer_load_act(idx: usize, val: i8) {
    if let Ok(mut g) = accel().lock() {
        if idx < 784 {
            g.input[idx] = val;
        }
    }
}

#[inline]
pub(crate) fn run_inference() {
    if let Ok(mut g) = accel().lock() {
        g.logits = WEIGHTS.forward(&g.input);
    }
}

#[inline]
pub(crate) fn read_logit(idx: usize) -> i32 {
    accel()
        .lock()
        .map(|g| g.logits[idx.min(9)])
        .unwrap_or(0)
}

impl MnistWeights {
    fn forward(&self, input: &[i8; 784]) -> [i32; 10] {
        let fc1_output = matmul_sym::<256, 784>(&self.fc1_weights, input, self.fc1_scale_q16);
        let mut fc1_activations = int32_to_int8_arr::<256>(&fc1_output);
        relu8::<256>(&mut fc1_activations);

        let fc2_output = matmul_sym::<128, 256>(&self.fc2_weights, &fc1_activations, self.fc2_scale_q16);
        let mut fc2_activations = int32_to_int8_arr::<128>(&fc2_output);
        relu8::<128>(&mut fc2_activations);

        matmul_sym::<10, 128>(&self.fc3_weights, &fc2_activations, self.fc3_scale_q16)
    }
}

fn matmul_sym<const ROWS: usize, const COLS: usize>(
    weights: &[[i8; COLS]; ROWS],
    input: &[i8; COLS],
    scale_q16: i32,
) -> [i32; ROWS] {
    let mut out = [0i32; ROWS];
    for i in 0..ROWS {
        let mut sum: i32 = 0;
        for j in 0..COLS {
            sum += weights[i][j] as i32 * input[j] as i32;
        }
        let scaled = (sum as i64 * scale_q16 as i64) >> Q16_SHIFT;
        out[i] = scaled as i32;
    }
    out
}

fn relu8<const N: usize>(data: &mut [i8; N]) {
    for val in data.iter_mut() {
        if *val < 0 {
            *val = 0;
        }
    }
}

fn int32_to_int8_arr<const N: usize>(input: &[i32; N]) -> [i8; N] {
    let max_abs = input.iter().fold(0i32, |acc, &x| acc.max(x.abs()));
    if max_abs == 0 {
        return [0i8; N];
    }
    let mut shift = 0;
    let mut max_val = max_abs;
    while max_val > 127 && shift < 31 {
        max_val >>= 1;
        shift += 1;
    }
    let mut result = [0i8; N];
    for i in 0..N {
        result[i] = (input[i] >> shift).clamp(-128, 127) as i8;
    }
    result
}
