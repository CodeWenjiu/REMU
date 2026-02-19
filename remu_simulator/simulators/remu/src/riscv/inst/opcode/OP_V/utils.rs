//! Shared vector execution helpers for OP-V sub-opcodes (element loop, mask compare, etc.).

use remu_types::isa::{
    extension_v::VExtensionConfig,
    reg::{VectorCsrState, VrState},
    RvIsa,
};

/// Number of register groups nf (1/2/4/8) from vlmul.
#[inline(always)]
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
/// Unmasked: no v0 read, dst zeroed, mask_bit true, dst_val 0; Masked: v0 mask and read dst.
#[inline(always)]
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
    let mut nf = nf_max.min(32_usize.saturating_sub(rd));
    if let Some(r2) = rs2 {
        nf = nf.min(32_usize.saturating_sub(r2));
    }
    let total_elems = (nf * vlenb) / sew_bytes;
    let n = vl.min(total_elems as u32);

    let v0 = match mode {
        VectorElementLoopMode::Unmasked => None,
        VectorElementLoopMode::Masked => Some(state.reg.vr.raw_read(0).to_vec()),
    };

    for r in 0..nf {
        let src_chunk = rs2.map(|reg| state.reg.vr.raw_read(reg + r).to_vec());
        let mut dst_chunk: Vec<u8> = match mode {
            VectorElementLoopMode::Unmasked => vec![0u8; vlenb],
            VectorElementLoopMode::Masked => state.reg.vr.raw_read(rd + r).to_vec(),
        };
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
            let dst_val = match mode {
                VectorElementLoopMode::Unmasked => 0,
                VectorElementLoopMode::Masked => match sew_bytes {
                    1 => dst_chunk[off] as u64,
                    2 => u16::from_le_bytes(dst_chunk[off..off + 2].try_into().unwrap()) as u64,
                    4 => u32::from_le_bytes(dst_chunk[off..off + 4].try_into().unwrap()) as u64,
                    8 => u64::from_le_bytes(dst_chunk[off..off + 8].try_into().unwrap()),
                    _ => 0,
                },
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

/// Mask compare vi: vd[i] = (vs2[i] == simm5 sign-extended to SEW); result written as mask bits in vd.
#[inline(always)]
pub(crate) fn vector_mask_cmp_vi<P, C>(
    ctx: &mut C,
    vd: usize,
    vs2: usize,
    simm5: u32,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let state = ctx.state_mut();
    let vl = state.reg.csr.vector.vl();
    let vtype = state.reg.csr.vector.vtype();
    let vlmul = vtype & 0x7;
    let vsew = (vtype >> 3) & 0x7;
    let sew_bytes = 1 << (vsew & 0x3);
    let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
    let nf = nf_from_vlmul(vlmul).min(32_usize.saturating_sub(vs2));
    let total_elems = (nf * vlenb) / sew_bytes;
    let n = vl.min(total_elems as u32) as usize;

    let mut vd_buf = state.reg.vr.raw_read(vd).to_vec();

    for i in 0..n {
        let byte_offset = (i as usize) * sew_bytes;
        let reg_i = byte_offset / vlenb;
        let off = byte_offset % vlenb;
        let chunk = state.reg.vr.raw_read(vs2 + reg_i);
        let vs2_val = match sew_bytes {
            1 => chunk[off] as u64,
            2 => u16::from_le_bytes(chunk[off..off + 2].try_into().unwrap()) as u64,
            4 => u32::from_le_bytes(chunk[off..off + 4].try_into().unwrap()) as u64,
            8 => u64::from_le_bytes(chunk[off..off + 8].try_into().unwrap()),
            _ => 0,
        };
        let imm_sext = match sew_bytes {
            1 => (simm5 as i8) as u8 as u64,
            2 => (simm5 as i16) as u16 as u64,
            4 => (simm5 as i32) as u32 as u64,
            8 => (simm5 as i64) as u64,
            _ => 0,
        };
        let bit = (vs2_val == imm_sext) as u8;
        let byte_idx = i / 8;
        let bit_idx = i % 8;
        if bit != 0 {
            vd_buf[byte_idx] |= 1u8 << bit_idx;
        } else {
            vd_buf[byte_idx] &= !(1u8 << bit_idx);
        }
    }

    state.reg.vr.raw_write(vd, &vd_buf);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}
