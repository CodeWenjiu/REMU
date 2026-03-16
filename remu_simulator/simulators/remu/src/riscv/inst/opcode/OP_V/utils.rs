//! Shared vector execution helpers for OP-V sub-opcodes (element loop, mask compare, etc.).

use remu_types::isa::reg::VrState;

use super::context::VContext;

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
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();
    if rd + vctx.nf > 32 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    if let Some(r2) = rs2 {
        if r2 + vctx.nf > 32 {
            return Err(remu_state::StateError::BusError(Box::new(
                remu_state::bus::BusError::unmapped(0),
            )));
        }
    }
    let n = vctx.n_elems();

    let v0 = match mode {
        VectorElementLoopMode::Unmasked => None,
        VectorElementLoopMode::Masked => Some(state.reg.vr.raw_read(0).to_vec()),
    };

    for r in 0..vctx.nf {
        let src_chunk = rs2.map(|reg| state.reg.vr.raw_read(reg + r).to_vec());
        let mut dst_chunk: Vec<u8> = state.reg.vr.raw_read(rd + r).to_vec();
        let start_elem = (r * vctx.vlenb) / vctx.sew_bytes;
        let end_elem = ((r + 1) * vctx.vlenb) / vctx.sew_bytes;
        let loop_start = (start_elem as u32).min(n);
        let loop_end = (end_elem as u32).min(n);

        for i in loop_start..loop_end {
            let (_, _, off) = vctx.elem_layout(i as usize);
            let mask_bit = match mode {
                VectorElementLoopMode::Unmasked => true,
                VectorElementLoopMode::Masked => {
                    let v0 = v0.as_ref().unwrap();
                    super::mask_bit(v0, i as usize)
                }
            };
            let src_val = src_chunk.as_ref().map(|chunk| vctx.sew.read_u(chunk, off));
            let dst_val = vctx.sew.read_u(&dst_chunk, off);
            let res = op(i, vctx.sew_bytes, src_val, mask_bit, dst_val);
            vctx.sew.write(&mut dst_chunk, off, res);
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
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();
    if rd + vctx.nf > 32 || rs1 + vctx.nf > 32 || rs2 + vctx.nf > 32 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    let n = vctx.n_elems();

    let v0 = match mode {
        VectorElementLoopMode::Unmasked => None,
        VectorElementLoopMode::Masked => Some(state.reg.vr.raw_read(0).to_vec()),
    };

    for r in 0..vctx.nf {
        let src1_chunk = state.reg.vr.raw_read(rs1 + r).to_vec();
        let src2_chunk = state.reg.vr.raw_read(rs2 + r).to_vec();
        let mut dst_chunk: Vec<u8> = state.reg.vr.raw_read(rd + r).to_vec();
        let start_elem = (r * vctx.vlenb) / vctx.sew_bytes;
        let end_elem = ((r + 1) * vctx.vlenb) / vctx.sew_bytes;
        let loop_start = (start_elem as u32).min(n);
        let loop_end = (end_elem as u32).min(n);

        for i in loop_start..loop_end {
            let (_, _, off) = vctx.elem_layout(i as usize);
            let mask_bit = match mode {
                VectorElementLoopMode::Unmasked => true,
                VectorElementLoopMode::Masked => {
                    let v0 = v0.as_ref().unwrap();
                    super::mask_bit(v0, i as usize)
                }
            };
            let src1_val = vctx.sew.read_u(&src1_chunk, off);
            let src2_val = vctx.sew.read_u(&src2_chunk, off);
            let dst_val = vctx.sew.read_u(&dst_chunk, off);
            let res = op(i, vctx.sew_bytes, src1_val, src2_val, mask_bit, dst_val);
            vctx.sew.write(&mut dst_chunk, off, res);
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
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();
    let vd = decoded.rd as usize;
    let vs2 = decoded.rs2 as usize;
    let raw_imm5 = decoded.imm & 0x1F;
    let simm_sext = ((raw_imm5 << 27) as i32 >> 27) as i64;
    let vm = (decoded.imm >> 8) != 0;

    let nf = vctx.nf.min(32_usize.saturating_sub(vs2));
    let total_elems = (nf * vctx.vlenb) / vctx.sew_bytes;
    let n = vctx.vl.min(total_elems as u32) as usize;

    let v0 = state.reg.vr.raw_read(0).to_vec();
    let mut vd_buf = state.reg.vr.raw_read(vd).to_vec();

    for i in 0..n {
        let active = vm || super::mask_bit(&v0, i);
        let result_bit = if active {
            let (_, reg_i, off) = vctx.elem_layout(i);
            let chunk = state.reg.vr.raw_read(vs2 + reg_i);
            let vs2_val = vctx.sew.read_i(chunk, off);
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
