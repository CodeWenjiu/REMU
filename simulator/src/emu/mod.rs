use std::ops::Range;

remu_macro::mod_flat!(emu);
remu_macro::mod_pub!(isa);

fn extract_bits(input: u32, range: Range<u8>) -> u32 {
    let mask = (1 << (range.end - range.start + 1)) - 1;
    (input >> range.start) & mask
}

fn sig_extend(imm: u32, size: u8) -> u32 {
    if imm & (1 << (size - 1)) != 0 {
        imm | !((1 << size) - 1)
    } else {
        imm
    }
}
