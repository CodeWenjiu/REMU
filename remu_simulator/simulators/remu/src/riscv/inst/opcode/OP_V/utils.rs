//! Shared vector execution helpers for OP-V sub-opcodes (element loop, mask compare, etc.).

use remu_types::isa::{
    extension_v::VExtensionConfig,
    reg::{VectorCsrState, VrState},
    RvIsa,
};

/// VLMAX in elements (standard formula, valid for fractional LMUL).
pub(crate) fn calculate_vlmax(vlenb: u32, vtype: u32) -> u32 {
    let vsew = (vtype >> 3) & 0x7;
    let vlmul = vtype & 0x7;
    let lmul_shift: i8 = match vlmul {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        5 => -3,
        6 => -2,
        7 => -1,
        _ => return 0,
    };
    let sew_shift: i8 = match vsew {
        0..=3 => -(vsew as i8),
        _ => return 0,
    };
    let total_shift = lmul_shift + sew_shift;
    if total_shift >= 0 {
        vlenb << total_shift
    } else {
        vlenb >> (-total_shift)
    }
}

/// Number of register groups nf (1/2/4/8) from vlmul.
pub(crate) fn nf_from_vlmul(vlmul: u32) -> usize {
    match vlmul & 0x7 {
        0 => 1,
        1 => 2,
        2 => 4,
        3 => 8,
        _ => 1,
    }
}

/// Vector element loop mode; dispatched via match inside.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VectorElementLoopMode {
    Unmasked,
    Masked,
}

/// Single entry for vector element loop. Iterates over vl/vtype and calls
/// `op(elem_idx, sew_bytes, src_val, mask_bit, dst_val)` per element, writing result to rd.
/// Unmasked: no v0 read, mask_bit true; Masked: v0 mask. Tail elements and vl=0 are preserved
/// by always reading rd before the loop (no zero-init).
pub(crate) fn vector_element_loop<P, C, F>(
    ctx: &mut C,
    rd: usize,
    rs2: Option<usize>,
    mode: VectorElementLoopMode,
    mut op: F,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
    F: FnMut(u32, usize, Option<u64>, bool, u64) -> u64,
{
    let state = ctx.state_mut();
    let vl = state.reg.csr.vector.vl();
    let vtype = state.reg.csr.vector.vtype();
    let vlmul = vtype & 0x7;
    let vsew = (vtype >> 3) & 0x7;
    let sew_bytes = 1 << (vsew & 0x3);
    let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
    let nf_max = nf_from_vlmul(vlmul);
    if rd + nf_max > 32 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    if let Some(r2) = rs2 {
        if r2 + nf_max > 32 {
            return Err(remu_state::StateError::BusError(Box::new(
                remu_state::bus::BusError::unmapped(0),
            )));
        }
    }
    let nf = nf_max;
    let total_elems = (nf * vlenb) / sew_bytes;
    let n = vl.min(total_elems as u32);

    let v0 = match mode {
        VectorElementLoopMode::Unmasked => None,
        VectorElementLoopMode::Masked => Some(state.reg.vr.raw_read(0).to_vec()),
    };

    for r in 0..nf {
        let src_chunk = rs2.map(|reg| state.reg.vr.raw_read(reg + r).to_vec());
        let mut dst_chunk: Vec<u8> = state.reg.vr.raw_read(rd + r).to_vec();
        let start_elem = (r * vlenb) / sew_bytes;
        let end_elem = ((r + 1) * vlenb) / sew_bytes;
        let loop_start = (start_elem as u32).min(n);
        let loop_end = (end_elem as u32).min(n);

        for i in loop_start..loop_end {
            let off = ((i as usize) * sew_bytes) % vlenb;
            let mask_bit = match mode {
                VectorElementLoopMode::Unmasked => true,
                VectorElementLoopMode::Masked => {
                    let v0 = v0.as_ref().unwrap();
                    (v0[(i as usize) / 8] >> (i % 8)) & 1 != 0
                }
            };
            let src_val = src_chunk.as_ref().map(|chunk| match sew_bytes {
                1 => chunk[off] as u64,
                2 => u16::from_le_bytes(chunk[off..off + 2].try_into().unwrap()) as u64,
                4 => u32::from_le_bytes(chunk[off..off + 4].try_into().unwrap()) as u64,
                8 => u64::from_le_bytes(chunk[off..off + 8].try_into().unwrap()),
                _ => 0,
            });
            let dst_val = match sew_bytes {
                1 => dst_chunk[off] as u64,
                2 => u16::from_le_bytes(dst_chunk[off..off + 2].try_into().unwrap()) as u64,
                4 => u32::from_le_bytes(dst_chunk[off..off + 4].try_into().unwrap()) as u64,
                8 => u64::from_le_bytes(dst_chunk[off..off + 8].try_into().unwrap()),
                _ => 0,
            };
            let res = op(i, sew_bytes, src_val, mask_bit, dst_val);
            match sew_bytes {
                1 => dst_chunk[off] = res as u8,
                2 => dst_chunk[off..off + 2].copy_from_slice(&(res as u16).to_le_bytes()),
                4 => dst_chunk[off..off + 4].copy_from_slice(&(res as u32).to_le_bytes()),
                8 => dst_chunk[off..off + 8].copy_from_slice(&res.to_le_bytes()),
                _ => {}
            }
        }
        state.reg.vr.raw_write(rd + r, &dst_chunk);
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// Two-source vector element loop. Iterates over vl/vtype, reads vs1 and vs2 per element,
/// calls `op(elem_idx, sew_bytes, src1_val, src2_val, mask_bit, dst_val)` and writes result to rd.
pub(crate) fn vector_element_loop_vv<P, C, F>(
    ctx: &mut C,
    rd: usize,
    rs1: usize,
    rs2: usize,
    mode: VectorElementLoopMode,
    mut op: F,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
    F: FnMut(u32, usize, u64, u64, bool, u64) -> u64,
{
    let state = ctx.state_mut();
    let vl = state.reg.csr.vector.vl();
    let vtype = state.reg.csr.vector.vtype();
    let vlmul = vtype & 0x7;
    let vsew = (vtype >> 3) & 0x7;
    let sew_bytes = 1 << (vsew & 0x3);
    let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
    let nf_max = nf_from_vlmul(vlmul);
    if rd + nf_max > 32 || rs1 + nf_max > 32 || rs2 + nf_max > 32 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    let nf = nf_max;
    let total_elems = (nf * vlenb) / sew_bytes;
    let n = vl.min(total_elems as u32);

    let v0 = match mode {
        VectorElementLoopMode::Unmasked => None,
        VectorElementLoopMode::Masked => Some(state.reg.vr.raw_read(0).to_vec()),
    };

    for r in 0..nf {
        let src1_chunk = state.reg.vr.raw_read(rs1 + r).to_vec();
        let src2_chunk = state.reg.vr.raw_read(rs2 + r).to_vec();
        let mut dst_chunk: Vec<u8> = state.reg.vr.raw_read(rd + r).to_vec();
        let start_elem = (r * vlenb) / sew_bytes;
        let end_elem = ((r + 1) * vlenb) / sew_bytes;
        let loop_start = (start_elem as u32).min(n);
        let loop_end = (end_elem as u32).min(n);

        for i in loop_start..loop_end {
            let off = ((i as usize) * sew_bytes) % vlenb;
            let mask_bit = match mode {
                VectorElementLoopMode::Unmasked => true,
                VectorElementLoopMode::Masked => {
                    let v0 = v0.as_ref().unwrap();
                    (v0[(i as usize) / 8] >> (i % 8)) & 1 != 0
                }
            };
            let src1_val = match sew_bytes {
                1 => src1_chunk[off] as u64,
                2 => u16::from_le_bytes(src1_chunk[off..off + 2].try_into().unwrap()) as u64,
                4 => u32::from_le_bytes(src1_chunk[off..off + 4].try_into().unwrap()) as u64,
                8 => u64::from_le_bytes(src1_chunk[off..off + 8].try_into().unwrap()),
                _ => 0,
            };
            let src2_val = match sew_bytes {
                1 => src2_chunk[off] as u64,
                2 => u16::from_le_bytes(src2_chunk[off..off + 2].try_into().unwrap()) as u64,
                4 => u32::from_le_bytes(src2_chunk[off..off + 4].try_into().unwrap()) as u64,
                8 => u64::from_le_bytes(src2_chunk[off..off + 8].try_into().unwrap()),
                _ => 0,
            };
            let dst_val = match sew_bytes {
                1 => dst_chunk[off] as u64,
                2 => u16::from_le_bytes(dst_chunk[off..off + 2].try_into().unwrap()) as u64,
                4 => u32::from_le_bytes(dst_chunk[off..off + 4].try_into().unwrap()) as u64,
                8 => u64::from_le_bytes(dst_chunk[off..off + 8].try_into().unwrap()),
                _ => 0,
            };
            let res = op(i, sew_bytes, src1_val, src2_val, mask_bit, dst_val);
            match sew_bytes {
                1 => dst_chunk[off] = res as u8,
                2 => dst_chunk[off..off + 2].copy_from_slice(&(res as u16).to_le_bytes()),
                4 => dst_chunk[off..off + 4].copy_from_slice(&(res as u32).to_le_bytes()),
                8 => dst_chunk[off..off + 8].copy_from_slice(&res.to_le_bytes()),
                _ => {}
            }
        }
        state.reg.vr.raw_write(rd + r, &dst_chunk);
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// Mask compare vi with closure. decoded.imm: low 5 bits = simm5, bit 8 = vm.
/// Result written as 1 bit per element (bit-packed) in vd.
pub(crate) fn vector_mask_cmp_vi<P, C, F>(
    ctx: &mut C,
    decoded: &crate::riscv::inst::DecodedInst,
    cmp_op: F,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
    F: Fn(i64, i64) -> bool,
{
    let state = ctx.state_mut();
    let vd = decoded.rd as usize;
    let vs2 = decoded.rs2 as usize;
    let raw_imm5 = decoded.imm & 0x1F;
    let simm_sext = ((raw_imm5 << 27) as i32 >> 27) as i64;
    let vm = (decoded.imm >> 8) != 0;

    let vl = state.reg.csr.vector.vl();
    let vtype = state.reg.csr.vector.vtype();
    let vlmul = vtype & 0x7;
    let vsew = (vtype >> 3) & 0x7;
    let sew_bytes = 1 << (vsew & 0x3);
    let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
    let nf = nf_from_vlmul(vlmul).min(32_usize.saturating_sub(vs2));
    let total_elems = (nf * vlenb) / sew_bytes;
    let n = vl.min(total_elems as u32) as usize;

    let v0 = state.reg.vr.raw_read(0).to_vec();
    let mut vd_buf = state.reg.vr.raw_read(vd).to_vec();

    for i in 0..n {
        let active = vm || ((v0[i / 8] >> (i % 8)) & 1 != 0);
        let result_bit = if active {
            let byte_offset = i * sew_bytes;
            let reg_i = byte_offset / vlenb;
            let off = byte_offset % vlenb;
            let chunk = state.reg.vr.raw_read(vs2 + reg_i);
            let vs2_val = match sew_bytes {
                1 => chunk[off] as i8 as i64,
                2 => i16::from_le_bytes(chunk[off..off + 2].try_into().unwrap()) as i64,
                4 => i32::from_le_bytes(chunk[off..off + 4].try_into().unwrap()) as i64,
                8 => i64::from_le_bytes(chunk[off..off + 8].try_into().unwrap()),
                _ => 0,
            };
            cmp_op(vs2_val, simm_sext)
        } else {
            false
        };
        let byte_idx = i / 8;
        let bit_idx = i % 8;
        if result_bit {
            vd_buf[byte_idx] |= 1u8 << bit_idx;
        } else {
            vd_buf[byte_idx] &= !(1u8 << bit_idx);
        }
    }

    state.reg.vr.raw_write(vd, &vd_buf);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}
