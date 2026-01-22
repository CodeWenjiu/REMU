#[inline(always)]
pub(crate) fn opcode(inst: u32) -> u32 {
    inst & 0x7F
}

#[inline(always)]
pub(crate) fn rd(inst: u32) -> u8 {
    ((inst >> 7) & 0x1F) as u8
}

#[inline(always)]
pub(crate) fn rs1(inst: u32) -> u8 {
    ((inst >> 15) & 0x1F) as u8
}

#[inline(always)]
pub(crate) fn rs2(inst: u32) -> u8 {
    ((inst >> 20) & 0x1F) as u8
}

#[inline(always)]
pub(crate) fn funct3(inst: u32) -> u32 {
    (inst >> 12) & 0x07
}

#[inline(always)]
pub(crate) fn funct7(inst: u32) -> u32 {
    (inst >> 25) & 0x7F
}

#[inline(always)]
fn sign_extend(val: u32, bits: u32) -> u32 {
    let shift = 32 - bits;
    ((val << shift) as i32 >> shift) as u32
}

// === I-Type (12-bit signed) ===
// format: inst[31:20] -> imm[11:0]
#[inline(always)]
pub(crate) fn imm_i(inst: u32) -> u32 {
    (inst as i32 >> 20) as u32
}

// === S-Type (12-bit signed) ===
// format: inst[31:25] | inst[11:7] -> imm[11:0]
#[inline(always)]
pub(crate) fn imm_s(inst: u32) -> u32 {
    let hi = (inst as i32 >> 25) << 5;
    let lo = (inst >> 7) & 0x1F;
    hi as u32 | lo
}

// === B-Type (13-bit signed, bit 0 is always 0) ===
// format: inst[31] | inst[7] | inst[30:25] | inst[11:8] -> imm[12:1]
#[inline(always)]
pub(crate) fn imm_b(inst: u32) -> u32 {
    let bit_12 = (inst >> 31) & 1;
    let bit_11 = (inst >> 7) & 1;
    let bits_10_5 = (inst >> 25) & 0x3F;
    let bits_4_1 = (inst >> 8) & 0x0F;

    let raw = (bit_12 << 12) | (bit_11 << 11) | (bits_10_5 << 5) | (bits_4_1 << 1);

    sign_extend(raw, 13)
}

// === U-Type (20-bit upper) ===
// format: inst[31:12] -> imm[31:12]
#[inline(always)]
pub(crate) fn imm_u(inst: u32) -> u32 {
    inst & 0xFFFFF000
}

// === J-Type (21-bit signed, bit 0 is always 0) ===
// format: inst[31] | inst[19:12] | inst[20] | inst[30:21] -> imm[20:1]
#[inline(always)]
pub(crate) fn imm_j(inst: u32) -> u32 {
    let bit_20 = (inst >> 31) & 1;
    let bits_19_12 = (inst >> 12) & 0xFF;
    let bit_11 = (inst >> 20) & 1;
    let bits_10_1 = (inst >> 21) & 0x3FF;

    let raw = (bit_20 << 20) | (bits_19_12 << 12) | (bit_11 << 11) | (bits_10_1 << 1);

    sign_extend(raw, 21)
}

// === CSR-Type (Immediate form, 5-bit zero-extended) ===
// format: inst[19:15] (uimm)
#[allow(unused)]
#[inline(always)]
pub(crate) fn imm_z(inst: u32) -> u32 {
    (inst >> 15) & 0x1F
}
