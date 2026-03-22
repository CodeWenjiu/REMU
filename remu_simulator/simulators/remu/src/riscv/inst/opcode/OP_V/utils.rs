//! Shared vector execution helpers for OP-V sub-opcodes (element loop, mask compare, etc.).

use remu_types::isa::reg::{RegAccess, VectorCsrState, VrState};

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

/// Mask compare: vs2[i] cmp scalar -> 1 bit per element. Unified for vi (simm5) and vx (GPR).
/// scalar: sign-extended compare operand. vm from decoded.imm bit 8.
pub(crate) fn vector_mask_cmp<P, C, F>(
    ctx: &mut C,
    vd: usize,
    vs2: usize,
    scalar: i64,
    vm: bool,
    cmp_op: F,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
    F: Fn(i64, i64) -> bool,
{
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();

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
            cmp_op(vs2_val, scalar)
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

/// Mask compare vector-vector (`vmsne.vv`, etc.): `vd[i] = cmp(vs1[i], vs2[i])` as mask bit.
/// `vm==true`: unmasked; else inactive elements (`v0[i]==0`) leave `vd[i]` unchanged (Spike `VI_LOOP_ELEMENT_SKIP`).
/// Only elements `i >= vstart` are processed.
pub(crate) fn vector_mask_cmp_vv<P, C, F>(
    ctx: &mut C,
    vd: usize,
    vs1: usize,
    vs2: usize,
    vm: bool,
    cmp_op: F,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
    F: Fn(i64, i64) -> bool,
{
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();
    let vstart = state.reg.csr.vector.vstart() as usize;
    let nf = vctx
        .nf
        .min(32_usize.saturating_sub(vs1))
        .min(32_usize.saturating_sub(vs2));
    let total_elems = (nf * vctx.vlenb) / vctx.sew_bytes;
    let n = vctx.vl.min(total_elems as u32) as usize;

    let v0 = state.reg.vr.raw_read(0).to_vec();
    let mut vd_buf = state.reg.vr.raw_read(vd).to_vec();

    let vs1_chunks: Vec<Vec<u8>> = (0..nf).map(|r| state.reg.vr.raw_read(vs1 + r).to_vec()).collect();
    let vs2_chunks: Vec<Vec<u8>> = (0..nf).map(|r| state.reg.vr.raw_read(vs2 + r).to_vec()).collect();

    for i in vstart..n {
        if !vm && !super::mask_bit(&v0, i) {
            continue;
        }
        let (_, reg_i, off) = vctx.elem_layout(i);
        let v1 = vctx.sew.read_i(&vs1_chunks[reg_i], off);
        let v2 = vctx.sew.read_i(&vs2_chunks[reg_i], off);
        let result_bit = cmp_op(v1, v2);
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

/// Slide: vd[i] = vs2[i + offset] if in range else 0. Covers vslidedown.
pub(crate) fn vector_slide<P, C>(
    ctx: &mut C,
    rd: usize,
    rs2: usize,
    offset: usize,
    mode: VectorElementLoopMode,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();
    let vlmax = vctx.vlmax as usize;
    let n = vctx.n_elems() as usize;
    let nf = vctx.nf
        .min(32_usize.saturating_sub(rd))
        .min(32_usize.saturating_sub(rs2))
        .max(1);

    let vs2_buf: Vec<Vec<u8>> = (0..nf).map(|r| state.reg.vr.raw_read(rs2 + r).to_vec()).collect();
    let mut vd_buf: Vec<Vec<u8>> = (0..nf).map(|r| state.reg.vr.raw_read(rd + r).to_vec()).collect();
    let v0 = match mode {
        VectorElementLoopMode::Unmasked => None,
        VectorElementLoopMode::Masked => Some(state.reg.vr.raw_read(0).to_vec()),
    };

    for i in 0..n {
        let mask = match &v0 {
            None => true,
            Some(v) => super::mask_bit(v, i),
        };
        if !mask {
            continue;
        }
        let src_idx = i + offset;
        let val = if src_idx < vlmax {
            let (_, reg_i, off) = vctx.elem_layout(src_idx);
            vctx.sew.read_u(&vs2_buf[reg_i], off)
        } else {
            0
        };
        let (_, dst_reg, dst_off) = vctx.elem_layout(i);
        vctx.sew.write(&mut vd_buf[dst_reg], dst_off, val);
    }

    for r in 0..nf {
        state.reg.vr.raw_write(rd + r, &vd_buf[r]);
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// Slide up (`vslideup.vi` / future `vslideup.vx`): for element indices `i` in `[vstart, vl)`,
/// when `vstart < offset && i < offset`, destination element is unchanged (Spike / resumable);
/// otherwise `vd[i] = vs2[i - offset]` when `(i - offset) < vlmax`, else `0`.
/// Requires `vd != vs2` (same as Spike `VI_CHECK_SLIDE(true)`).
pub(crate) fn vector_slide_up<P, C>(
    ctx: &mut C,
    rd: usize,
    rs2: usize,
    offset: usize,
    mode: VectorElementLoopMode,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();
    let vstart = state.reg.csr.vector.vstart() as usize;
    let vlmax = vctx.vlmax as usize;
    let n = vctx.n_elems() as usize;
    let nf = vctx
        .nf
        .min(32_usize.saturating_sub(rd))
        .min(32_usize.saturating_sub(rs2))
        .max(1);

    if rd == rs2 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }

    let vs2_buf: Vec<Vec<u8>> = (0..nf).map(|r| state.reg.vr.raw_read(rs2 + r).to_vec()).collect();
    let mut vd_buf: Vec<Vec<u8>> = (0..nf).map(|r| state.reg.vr.raw_read(rd + r).to_vec()).collect();
    let v0 = match mode {
        VectorElementLoopMode::Unmasked => None,
        VectorElementLoopMode::Masked => Some(state.reg.vr.raw_read(0).to_vec()),
    };

    for i in vstart..n {
        let mask = match &v0 {
            None => true,
            Some(v) => super::mask_bit(v, i),
        };
        if !mask {
            continue;
        }
        if vstart < offset && i < offset {
            continue;
        }
        let src_idx = i - offset;
        let val = if src_idx < vlmax {
            let (_, reg_i, off) = vctx.elem_layout(src_idx);
            vctx.sew.read_u(&vs2_buf[reg_i], off)
        } else {
            0
        };
        let (_, dst_reg, dst_off) = vctx.elem_layout(i);
        vctx.sew.write(&mut vd_buf[dst_reg], dst_off, val);
    }

    for r in 0..nf {
        state.reg.vr.raw_write(rd + r, &vd_buf[r]);
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// vslide1down.vx: for active i in [0, vl), vd[i] = vs2[i+1] if i != vl-1 else (rs1 truncated to SEW, unsigned).
pub(crate) fn vector_slide1down_vx<P, C>(
    ctx: &mut C,
    rd: usize,
    rs2: usize,
    rs1_x: u32,
    mode: VectorElementLoopMode,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();
    let vlmax = vctx.vlmax as usize;
    let n = vctx.n_elems() as usize;
    let vl = vctx.vl;
    let nf = vctx
        .nf
        .min(32_usize.saturating_sub(rd))
        .min(32_usize.saturating_sub(rs2))
        .max(1);

    let vs2_buf: Vec<Vec<u8>> = (0..nf).map(|r| state.reg.vr.raw_read(rs2 + r).to_vec()).collect();
    let mut vd_buf: Vec<Vec<u8>> = (0..nf).map(|r| state.reg.vr.raw_read(rd + r).to_vec()).collect();
    let v0 = match mode {
        VectorElementLoopMode::Unmasked => None,
        VectorElementLoopMode::Masked => Some(state.reg.vr.raw_read(0).to_vec()),
    };

    let scalar_u = rs1_x as u64;
    let scalar_for_sew = match vctx.sew_bytes {
        1 => scalar_u & 0xff,
        2 => scalar_u & 0xffff,
        4 => scalar_u & 0xffff_ffff,
        8 => scalar_u,
        _ => scalar_u & 0xffff_ffff,
    };

    let last_i = vl.wrapping_sub(1);

    for i in 0..n {
        let mask = match &v0 {
            None => true,
            Some(v) => super::mask_bit(v, i),
        };
        if !mask {
            continue;
        }
        let val = if (i as u32) == last_i {
            scalar_for_sew
        } else {
            let src_idx = i + 1;
            if src_idx < vlmax {
                let (_, reg_i, off) = vctx.elem_layout(src_idx);
                vctx.sew.read_u(&vs2_buf[reg_i], off)
            } else {
                0
            }
        };
        let (_, dst_reg, dst_off) = vctx.elem_layout(i);
        vctx.sew.write(&mut vd_buf[dst_reg], dst_off, val);
    }

    for r in 0..nf {
        state.reg.vr.raw_write(rd + r, &vd_buf[r]);
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// Extend vf2: read narrow (sew/2), write wide (sew). signed=true => sext, false => zext.
pub(crate) fn vector_extend_vf2<P, C>(
    ctx: &mut C,
    vd: usize,
    vs2: usize,
    vm: bool,
    signed: bool,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();
    let src_sew = vctx.sew_bytes / 2;
    if src_sew == 0 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    let nf = vctx
        .nf
        .min(32_usize.saturating_sub(vd))
        .min(32_usize.saturating_sub(vs2));
    let total_elems = (nf * vctx.vlenb) / vctx.sew_bytes;
    let n = vctx.vl.min(total_elems as u32) as usize;
    let v0 = state.reg.vr.raw_read(0).to_vec();

    for r in 0..nf {
        let mut dst = state.reg.vr.raw_read(vd + r).to_vec();
        let start = (r * vctx.vlenb) / vctx.sew_bytes;
        let end = ((r + 1) * vctx.vlenb) / vctx.sew_bytes;
        for i in start..end.min(n) {
            if !vm && !super::mask_bit(&v0, i) {
                continue;
            }
            let src_byte_off = i * src_sew;
            let src_reg = src_byte_off / vctx.vlenb;
            let src_off = src_byte_off % vctx.vlenb;
            let src_chunk = state.reg.vr.raw_read(vs2 + src_reg);
            let mut val = [0u8; 8];
            if signed {
                match (src_sew, vctx.sew_bytes) {
                    (1, 2) => {
                        val[..2].copy_from_slice(&(src_chunk[src_off] as i8 as i16 as u16).to_le_bytes())
                    }
                    (2, 4) => val[..4].copy_from_slice(
                        &(i16::from_le_bytes(src_chunk[src_off..src_off + 2].try_into().unwrap())
                            as i32 as u32)
                            .to_le_bytes(),
                    ),
                    (4, 8) => val[..8].copy_from_slice(
                        &(i32::from_le_bytes(src_chunk[src_off..src_off + 4].try_into().unwrap())
                            as i64 as u64)
                            .to_le_bytes(),
                    ),
                    _ => continue,
                }
            } else {
                match (src_sew, vctx.sew_bytes) {
                    (1, 2) => {
                        val[..2].copy_from_slice(&(src_chunk[src_off] as u16).to_le_bytes())
                    }
                    (2, 4) => val[..4].copy_from_slice(
                        &(u16::from_le_bytes(src_chunk[src_off..src_off + 2].try_into().unwrap())
                            as u32)
                            .to_le_bytes(),
                    ),
                    (4, 8) => val[..8].copy_from_slice(
                        &(u32::from_le_bytes(src_chunk[src_off..src_off + 4].try_into().unwrap())
                            as u64)
                            .to_le_bytes(),
                    ),
                    _ => continue,
                }
            };
            let dst_off = (i * vctx.sew_bytes) % vctx.vlenb;
            dst[dst_off..dst_off + vctx.sew_bytes].copy_from_slice(&val[..vctx.sew_bytes]);
        }
        state.reg.vr.raw_write(vd + r, &dst);
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// Extend vf4: read narrow (sew/4), write wide (sew). signed=true => sext, false => zext.
pub(crate) fn vector_extend_vf4<P, C>(
    ctx: &mut C,
    vd: usize,
    vs2: usize,
    vm: bool,
    signed: bool,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();
    let src_sew = vctx.sew_bytes / 4;
    if src_sew == 0 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    let nf = vctx.nf
        .min(32_usize.saturating_sub(vd))
        .min(32_usize.saturating_sub(vs2));
    let total_elems = (nf * vctx.vlenb) / vctx.sew_bytes;
    let n = vctx.vl.min(total_elems as u32) as usize;
    let v0 = state.reg.vr.raw_read(0).to_vec();

    for r in 0..nf {
        let mut dst = state.reg.vr.raw_read(vd + r).to_vec();
        let start = (r * vctx.vlenb) / vctx.sew_bytes;
        let end = ((r + 1) * vctx.vlenb) / vctx.sew_bytes;
        for i in start..end.min(n) {
            if !vm && !super::mask_bit(&v0, i) {
                continue;
            }
            let src_byte_off = i * src_sew;
            let src_reg = src_byte_off / vctx.vlenb;
            let src_off = src_byte_off % vctx.vlenb;
            let src_chunk = state.reg.vr.raw_read(vs2 + src_reg);
            let mut val = [0u8; 8];
            if signed {
                match (src_sew, vctx.sew_bytes) {
                    (1, 4) => val[..4].copy_from_slice(&(src_chunk[src_off] as i8 as i32 as u32).to_le_bytes()),
                    (1, 8) => val[..8].copy_from_slice(&(src_chunk[src_off] as i8 as i64 as u64).to_le_bytes()),
                    (2, 4) => val[..4].copy_from_slice(&(i16::from_le_bytes(src_chunk[src_off..src_off + 2].try_into().unwrap()) as i32 as u32).to_le_bytes()),
                    (2, 8) => val[..8].copy_from_slice(&(i16::from_le_bytes(src_chunk[src_off..src_off + 2].try_into().unwrap()) as i64 as u64).to_le_bytes()),
                    _ => continue,
                }
            } else {
                match (src_sew, vctx.sew_bytes) {
                    (1, 4) => val[..4].copy_from_slice(&(src_chunk[src_off] as u32).to_le_bytes()),
                    (1, 8) => val[..8].copy_from_slice(&(src_chunk[src_off] as u64).to_le_bytes()),
                    (2, 4) => val[..4].copy_from_slice(&(u16::from_le_bytes(src_chunk[src_off..src_off + 2].try_into().unwrap()) as u32).to_le_bytes()),
                    (2, 8) => val[..8].copy_from_slice(&(u16::from_le_bytes(src_chunk[src_off..src_off + 2].try_into().unwrap()) as u64).to_le_bytes()),
                    _ => continue,
                }
            };
            let dst_off = (i * vctx.sew_bytes) % vctx.vlenb;
            dst[dst_off..dst_off + vctx.sew_bytes].copy_from_slice(&val[..vctx.sew_bytes]);
        }
        state.reg.vr.raw_write(vd + r, &dst);
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// Reduction: acc = vs1[0]; for i in 0..n { if active(i) { acc = acc + vs2[i] } }; vd[0] = acc.
pub(crate) fn vector_reduction<P, C, F>(
    ctx: &mut C,
    vd: usize,
    vs1: usize,
    vs2: usize,
    vm: bool,
    mut op: F,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
    F: FnMut(i64, i64) -> i64,
{
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();
    if vctx.vl == 0 {
        *state.reg.pc = state.reg.pc.wrapping_add(4);
        return Ok(());
    }
    let nf = vctx.nf.min(32_usize.saturating_sub(vs2));
    let total_elems = (nf * vctx.vlenb) / vctx.sew_bytes;
    let n = vctx.vl.min(total_elems as u32) as usize;
    let v0 = state.reg.vr.raw_read(0).to_vec();
    let mut acc = vctx.sew.read_i(state.reg.vr.raw_read(vs1), 0);

    for i in 0..n {
        if !vm && !super::mask_bit(&v0, i) {
            continue;
        }
        let (_, reg_i, off) = vctx.elem_layout(i);
        let val = vctx.sew.read_i(state.reg.vr.raw_read(vs2 + reg_i), off);
        acc = op(acc, val);
    }

    let mut vd_chunk = state.reg.vr.raw_read(vd).to_vec();
    vctx.sew.write(&mut vd_chunk, 0, acc as u64);
    state.reg.vr.raw_write(vd, &vd_chunk);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// Insert scalar into vd[0]. For vmv.s.x.
pub(crate) fn vector_insert_scalar<P, C>(
    ctx: &mut C,
    vd: usize,
    scalar: u32,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();
    let mut chunk = state.reg.vr.raw_read(vd).to_vec();
    vctx.sew.write(&mut chunk, 0, scalar as u64);
    state.reg.vr.raw_write(vd, &chunk);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// Extract first element of vs2 to GPR rd. For vmv.x.s.
pub(crate) fn vector_extract_scalar<P, C>(
    ctx: &mut C,
    rd: u8,
    vs2: usize,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();
    let chunk = state.reg.vr.raw_read(vs2);
    state.reg.gpr.raw_write(rd.into(), vctx.sew.read_i(chunk, 0) as u32);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// First set bit in mask: vfirst.m. Returns index or !0.
pub(crate) fn vector_first_mask<P, C>(
    ctx: &mut C,
    vd_reg: u8,
    vs2: usize,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();
    let chunk = state.reg.vr.raw_read(vs2);
    let mut pos = !0u32;
    for i in 0..vctx.vl {
        let (bi, b) = (i as usize / 8, i % 8);
        if bi < chunk.len() && (chunk[bi] >> b) & 1 != 0 {
            pos = i;
            break;
        }
    }
    state.reg.gpr.raw_write(vd_reg.into(), pos);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// `vcpop.m rd, vs2, vm`: count mask bits set in `vs2` for active elements.
/// Active when `vm` (unmasked) or `v0[i]==1`. Requires `vstart==0` (Spike).
pub(crate) fn vector_cpop_m<P, C>(
    ctx: &mut C,
    rd_gpr: u8,
    vs2: usize,
    vm: bool,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();
    if state.reg.csr.vector.vstart() != 0 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    let vs2_buf = state.reg.vr.raw_read(vs2).to_vec();
    let v0 = state.reg.vr.raw_read(0).to_vec();
    let n = vctx.vl as usize;

    let mut popcount: u32 = 0;
    for i in 0..n {
        let vs2_bit = super::mask_bit(&vs2_buf, i);
        let active = vm || super::mask_bit(&v0, i);
        if vs2_bit && active {
            popcount = popcount.wrapping_add(1);
        }
    }

    state.reg.gpr.raw_write(rd_gpr.into(), popcount);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// Mask binary: vd[i] = op(vs1[i], vs2[i]) per bit. For vmor.
pub(crate) fn vector_mask_binary<P, C, F>(
    ctx: &mut C,
    vd: usize,
    vs1: usize,
    vs2: usize,
    op: F,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
    F: Fn(u8, u8) -> u8,
{
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();
    let vl = vctx.vl as usize;
    let vs1_buf = state.reg.vr.raw_read(vs1);
    let vs2_buf = state.reg.vr.raw_read(vs2);
    let mut vd_buf = state.reg.vr.raw_read(vd).to_vec();

    for i in 0..vl {
        let (bi, b) = (i / 8, i % 8);
        let v = op((vs1_buf[bi] >> b) & 1, (vs2_buf[bi] >> b) & 1) & 1;
        if v != 0 {
            vd_buf[bi] |= 1 << b;
        } else {
            vd_buf[bi] &= !(1 << b);
        }
    }
    state.reg.vr.raw_write(vd, &vd_buf);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// Wide mul vv: vd[i] = vs1[i]*vs2[i] (+ vd[i] if accumulate). signed: signed mul, else unsigned.
pub(crate) fn vector_wide_mul_vv<P, C>(
    ctx: &mut C,
    vd: usize,
    vs1: usize,
    vs2: usize,
    vm: bool,
    signed: bool,
    accumulate: bool,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();
    if vctx.sew_bytes >= 4 || vctx.vlmul == 3 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    let (nf_src, nf_dst) = (vctx.nf, vctx.nf * 2);
    if vd % nf_dst != 0 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    super::vreg_check::check_regs(vd, nf_dst, Some((vs1, nf_src)), Some((vs2, nf_src)), true)?;

    let total_elems = (nf_src * vctx.vlenb) / vctx.sew_bytes;
    let n = vctx.vl.min(total_elems as u32) as usize;
    let dst_bytes = vctx.sew_bytes * 2;
    let v0 = state.reg.vr.raw_read(0).to_vec();

    for r in 0..nf_dst {
        let mut dst = state.reg.vr.raw_read(vd + r).to_vec();
        let start = (r * vctx.vlenb) / dst_bytes;
        let end = ((r + 1) * vctx.vlenb) / dst_bytes;
        for i in start..end.min(n) {
            if !vm && !super::mask_bit(&v0, i) {
                continue;
            }
            let (_, nr, no) = vctx.elem_layout(i);
            let (vs1_c, vs2_c) = (
                state.reg.vr.raw_read(vs1 + nr),
                state.reg.vr.raw_read(vs2 + nr),
            );
            let wo = (i * dst_bytes) % vctx.vlenb;
            let d_old = if accumulate {
                match dst_bytes {
                    2 => i16::from_le_bytes(dst[wo..wo + 2].try_into().unwrap()) as i64,
                    4 => i32::from_le_bytes(dst[wo..wo + 4].try_into().unwrap()) as i64,
                    8 => i64::from_le_bytes(dst[wo..wo + 8].try_into().unwrap()),
                    _ => 0,
                }
            } else {
                0
            };
            let res = if signed {
                let (s1, s2) = (vctx.sew.read_i(vs1_c, no), vctx.sew.read_i(vs2_c, no));
                s1.wrapping_mul(s2).wrapping_add(d_old)
            } else {
                let (s1, s2) = (vctx.sew.read_u(vs1_c, no), vctx.sew.read_u(vs2_c, no));
                (s1.wrapping_mul(s2).wrapping_add(d_old as u64)) as i64
            };
            match dst_bytes {
                2 => dst[wo..wo + 2].copy_from_slice(&(res as i16).to_le_bytes()),
                4 => dst[wo..wo + 4].copy_from_slice(&(res as i32).to_le_bytes()),
                8 => dst[wo..wo + 8].copy_from_slice(&res.to_le_bytes()),
                _ => {}
            }
        }
        state.reg.vr.raw_write(vd + r, &dst);
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// Wide mul vx: vd[i] = vs2[i] * scalar. signed mul.
pub(crate) fn vector_wide_mul_vx<P, C>(
    ctx: &mut C,
    vd: usize,
    vs2: usize,
    scalar: i64,
    vm: bool,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();
    if vctx.sew_bytes >= 4 || vctx.vlmul == 3 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    let (nf_src, nf_dst) = (vctx.nf, vctx.nf * 2);
    if vd % nf_dst != 0 || vd + nf_dst > 32 || vs2 + nf_src > 32 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    if !(vd + nf_dst <= vs2 || vs2 + nf_src <= vd) || vd == 0 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }

    let total_elems = (nf_src * vctx.vlenb) / vctx.sew_bytes;
    let n = vctx.vl.min(total_elems as u32) as usize;
    let dst_bytes = vctx.sew_bytes * 2;
    let v0 = state.reg.vr.raw_read(0).to_vec();

    for r in 0..nf_dst {
        let mut dst = state.reg.vr.raw_read(vd + r).to_vec();
        let start = (r * vctx.vlenb) / dst_bytes;
        let end = ((r + 1) * vctx.vlenb) / dst_bytes;
        for i in start..end.min(n) {
            if !vm && !super::mask_bit(&v0, i) {
                continue;
            }
            let (_, nr, no) = vctx.elem_layout(i);
            let vs2_c = state.reg.vr.raw_read(vs2 + nr);
            let s2 = vctx.sew.read_i(vs2_c, no);
            let res = s2.wrapping_mul(scalar);
            let wo = (i * dst_bytes) % vctx.vlenb;
            match dst_bytes {
                2 => dst[wo..wo + 2].copy_from_slice(&(res as i16).to_le_bytes()),
                4 => dst[wo..wo + 4].copy_from_slice(&(res as i32).to_le_bytes()),
                8 => dst[wo..wo + 8].copy_from_slice(&res.to_le_bytes()),
                _ => {}
            }
        }
        state.reg.vr.raw_write(vd + r, &dst);
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}
