//! CUS0-only backend: weights live in the remu simulator (`opcode/OP_WJCUS0`); on device, inference is
//! custom instructions only.

use remu_hal::println;

use super::{cus0_asm, normalize_and_quantize_input, MnistInference};

/// Zero-sized backend: issues [`cus0_asm::emit_pipeline`] after input normalization, then argmax
/// over the 10 `NN_LOAD` GPR words (interpreted as `i32` bit patterns).
pub struct Cus0Inference;

fn argmax_logits(bits: &[u32; 10]) -> usize {
    bits.iter()
        .enumerate()
        .max_by_key(|(_, v)| i32::from_ne_bytes(v.to_ne_bytes()))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

impl Cus0Inference {
    pub fn new() -> Self {
        println!("MNIST backend: CUS0 (NN_LOAD_ACT / NN_START / NN_LOAD).");
        Self
    }
}

impl MnistInference for Cus0Inference {
    fn infer(&self, input_image: &[u8]) -> usize {
        let normalized = normalize_and_quantize_input(input_image);
        let mut logits = [0u32; 10];
        cus0_asm::emit_pipeline(normalized.as_slice(), &mut logits);
        argmax_logits(&logits)
    }
}
