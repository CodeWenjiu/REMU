use std::ops::Range;

remu_macro::mod_flat!(emu, wrapper);
remu_macro::mod_pub!(isa);

fn sig_extend(imm: u32, size: u32) -> u32 {
    if imm & (1 << (size - 1)) != 0 {
        imm | !((1 << size) - 1)
    } else {
        imm
    }
}

fn extract_bits(input: u32, range: Range<u32>) -> u32 {
    let mask = (1u32.wrapping_shl(range.end - range.start + 1)).wrapping_sub(1);
    (input.wrapping_shr(range.start as u32)) & mask
}
