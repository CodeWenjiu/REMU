//! funct3 = 0b010: OP-MVV

use remu_types::isa::{
    RvIsa,
    extension_v::VExtensionConfig,
    reg::{RegAccess, VectorCsrState, VrState},
};

use crate::riscv::inst::{DecodedInst, opcode::OP_V::OpMvvInst};

use super::utils::{VectorElementLoopMode, nf_from_vlmul, vector_element_loop, vector_element_loop_vv};

fn vector_redsum_vs<P, C>(ctx: &mut C, decoded: &DecodedInst) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let state = ctx.state_mut();
    let vl = state.reg.csr.vector.vl();
    if vl == 0 {
        *state.reg.pc = state.reg.pc.wrapping_add(4);
        return Ok(());
    }

    let vd = decoded.rd as usize;
    let vs1 = decoded.rs1 as usize;
    let vs2 = decoded.rs2 as usize;
    let vm = decoded.imm != 0;

    let vtype = state.reg.csr.vector.vtype();
    let vlmul = vtype & 0x7;
    let vsew = (vtype >> 3) & 0x7;
    let sew_bytes = 1 << (vsew & 0x3);
    let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
    let nf = nf_from_vlmul(vlmul).min(32_usize.saturating_sub(vs2));
    let total_elems = (nf * vlenb) / sew_bytes;
    let n = vl.min(total_elems as u32) as usize;

    let v0 = state.reg.vr.raw_read(0).to_vec();

    let vs1_chunk = state.reg.vr.raw_read(vs1);
    let acc_sew = match sew_bytes {
        1 => vs1_chunk[0] as i8 as i64,
        2 => i16::from_le_bytes(vs1_chunk[0..2].try_into().unwrap()) as i64,
        4 => i32::from_le_bytes(vs1_chunk[0..4].try_into().unwrap()) as i64,
        8 => i64::from_le_bytes(vs1_chunk[0..8].try_into().unwrap()),
        _ => 0,
    };
    let mut acc = acc_sew;

    for i in 0..n {
        let active = vm || ((v0[i / 8] >> (i % 8)) & 1 != 0);
        if !active {
            continue;
        }
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
        acc = acc.wrapping_add(vs2_val);
    }

    let mut vd_chunk = state.reg.vr.raw_read(vd).to_vec();
    match sew_bytes {
        1 => vd_chunk[0] = (acc as i8) as u8,
        2 => vd_chunk[0..2].copy_from_slice(&(acc as i16).to_le_bytes()),
        4 => vd_chunk[0..4].copy_from_slice(&(acc as i32).to_le_bytes()),
        8 => vd_chunk[0..8].copy_from_slice(&acc.to_le_bytes()),
        _ => {}
    }
    state.reg.vr.raw_write(vd, &vd_chunk);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

fn vector_sext_vf4<P, C>(ctx: &mut C, decoded: &DecodedInst) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let state = ctx.state_mut();
    let vd = decoded.rd as usize;
    let vs2 = decoded.rs2 as usize;
    let vm = decoded.imm != 0;

    let vl = state.reg.csr.vector.vl();
    let vtype = state.reg.csr.vector.vtype();
    let vlmul = vtype & 0x7;
    let vsew = (vtype >> 3) & 0x7;
    let sew_bytes = 1 << (vsew & 0x3);
    let src_sew_bytes = sew_bytes / 4;
    if src_sew_bytes == 0 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
    let nf = nf_from_vlmul(vlmul)
        .min(32_usize.saturating_sub(vd))
        .min(32_usize.saturating_sub(vs2));
    let total_elems = (nf * vlenb) / sew_bytes;
    let n = vl.min(total_elems as u32) as usize;

    let v0 = state.reg.vr.raw_read(0).to_vec();

    for r in 0..nf {
        let mut dst_chunk = state.reg.vr.raw_read(vd + r).to_vec();
        let start_elem = (r * vlenb) / sew_bytes;
        let end_elem = ((r + 1) * vlenb) / sew_bytes;
        let loop_end = end_elem.min(n);

        for i in start_elem..loop_end {
            let active = vm || ((v0[i / 8] >> (i % 8)) & 1 != 0);
            if !active {
                continue;
            }
            let src_byte_off = i * src_sew_bytes;
            let src_reg = src_byte_off / vlenb;
            let src_off = src_byte_off % vlenb;
            let src_chunk = state.reg.vr.raw_read(vs2 + src_reg);
            let mut val = [0u8; 8];
            match (src_sew_bytes, sew_bytes) {
                (1, 4) => {
                    let b = src_chunk[src_off] as i8;
                    val[..4].copy_from_slice(&(b as i32 as u32).to_le_bytes());
                }
                (1, 8) => {
                    let b = src_chunk[src_off] as i8;
                    val[..8].copy_from_slice(&(b as i64 as u64).to_le_bytes());
                }
                (2, 4) => {
                    let w = i16::from_le_bytes(src_chunk[src_off..src_off + 2].try_into().unwrap());
                    val[..4].copy_from_slice(&(w as i32 as u32).to_le_bytes());
                }
                (2, 8) => {
                    let w = i16::from_le_bytes(src_chunk[src_off..src_off + 2].try_into().unwrap());
                    val[..8].copy_from_slice(&(w as i64 as u64).to_le_bytes());
                }
                _ => continue,
            }
            let dst_off = (i * sew_bytes) % vlenb;
            dst_chunk[dst_off..dst_off + sew_bytes].copy_from_slice(&val[..sew_bytes]);
        }
        state.reg.vr.raw_write(vd + r, &dst_chunk);
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

fn vector_wmacc_vv<P, C>(ctx: &mut C, decoded: &DecodedInst) -> Result<(), remu_state::StateError>
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
    let nf_src = nf_from_vlmul(vlmul);
    let nf_dst = nf_src * 2;

    if sew_bytes >= 4 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    if vlmul == 3 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    let vd = decoded.rd as usize;
    let vs1 = decoded.rs1 as usize;
    let vs2 = decoded.rs2 as usize;
    if vd % nf_dst != 0 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    if vd + nf_dst > 32 || vs1 + nf_src > 32 || vs2 + nf_src > 32 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    fn no_overlap(a: usize, na: usize, b: usize, nb: usize) -> bool {
        a + na <= b || b + nb <= a
    }
    if !no_overlap(vd, nf_dst, vs1, nf_src)
        || !no_overlap(vd, nf_dst, vs2, nf_src)
        || !no_overlap(vd, nf_dst, 0, 1)
    {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }

    let vm = decoded.imm != 0;
    let v0 = state.reg.vr.raw_read(0).to_vec();
    let total_elems_src = (nf_src * vlenb) / sew_bytes;
    let n = vl.min(total_elems_src as u32) as usize;
    let dst_bytes_per_elem = sew_bytes * 2;

    for r in 0..nf_dst {
        let mut dst_chunk = state.reg.vr.raw_read(vd + r).to_vec();
        let start_elem = (r * vlenb) / dst_bytes_per_elem;
        let end_elem = ((r + 1) * vlenb) / dst_bytes_per_elem;
        let loop_end = end_elem.min(n);
        for i in start_elem..loop_end {
            if !vm && ((v0[i / 8] >> (i % 8)) & 1 == 0) {
                continue;
            }
            let narrow_off = i * sew_bytes;
            let nr = narrow_off / vlenb;
            let no = narrow_off % vlenb;
            let vs1_chunk = state.reg.vr.raw_read(vs1 + nr);
            let vs2_chunk = state.reg.vr.raw_read(vs2 + nr);
            let s1_sext: i64 = match sew_bytes {
                1 => (vs1_chunk[no] as i8 as i16) as i64,
                2 => (i16::from_le_bytes(vs1_chunk[no..no + 2].try_into().unwrap()) as i32) as i64,
                _ => i32::from_le_bytes(vs1_chunk[no..no + 4].try_into().unwrap()) as i64,
            };
            let s2_sext: i64 = match sew_bytes {
                1 => (vs2_chunk[no] as i8 as i16) as i64,
                2 => (i16::from_le_bytes(vs2_chunk[no..no + 2].try_into().unwrap()) as i32) as i64,
                _ => i32::from_le_bytes(vs2_chunk[no..no + 4].try_into().unwrap()) as i64,
            };
            let wide_off = i * dst_bytes_per_elem;
            let wo = wide_off % vlenb;
            let d_old: i64 = match dst_bytes_per_elem {
                2 => i16::from_le_bytes(dst_chunk[wo..wo + 2].try_into().unwrap()) as i64,
                4 => i32::from_le_bytes(dst_chunk[wo..wo + 4].try_into().unwrap()) as i64,
                8 => i64::from_le_bytes(dst_chunk[wo..wo + 8].try_into().unwrap()),
                _ => 0,
            };
            let res = s1_sext.wrapping_mul(s2_sext).wrapping_add(d_old);
            match dst_bytes_per_elem {
                2 => dst_chunk[wo..wo + 2].copy_from_slice(&(res as i16).to_le_bytes()),
                4 => dst_chunk[wo..wo + 4].copy_from_slice(&(res as i32).to_le_bytes()),
                8 => dst_chunk[wo..wo + 8].copy_from_slice(&res.to_le_bytes()),
                _ => {}
            }
        }
        state.reg.vr.raw_write(vd + r, &dst_chunk);
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

fn vector_wmulu_vv<P, C>(ctx: &mut C, decoded: &DecodedInst) -> Result<(), remu_state::StateError>
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
    let nf_src = nf_from_vlmul(vlmul);
    let nf_dst = nf_src * 2;

    if sew_bytes >= 4 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    if vlmul == 3 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    let vd = decoded.rd as usize;
    let vs1 = decoded.rs1 as usize;
    let vs2 = decoded.rs2 as usize;
    if vd % nf_dst != 0 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    if vd + nf_dst > 32 || vs1 + nf_src > 32 || vs2 + nf_src > 32 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    fn no_overlap(a: usize, na: usize, b: usize, nb: usize) -> bool {
        a + na <= b || b + nb <= a
    }
    if !no_overlap(vd, nf_dst, vs1, nf_src)
        || !no_overlap(vd, nf_dst, vs2, nf_src)
        || !no_overlap(vd, nf_dst, 0, 1)
    {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }

    let vm = decoded.imm != 0;
    let v0 = state.reg.vr.raw_read(0).to_vec();
    let total_elems_src = (nf_src * vlenb) / sew_bytes;
    let n = vl.min(total_elems_src as u32) as usize;
    let dst_bytes_per_elem = sew_bytes * 2;

    for r in 0..nf_dst {
        let mut dst_chunk = state.reg.vr.raw_read(vd + r).to_vec();
        let start_elem = (r * vlenb) / dst_bytes_per_elem;
        let end_elem = ((r + 1) * vlenb) / dst_bytes_per_elem;
        let loop_end = end_elem.min(n);
        for i in start_elem..loop_end {
            if !vm && ((v0[i / 8] >> (i % 8)) & 1 == 0) {
                continue;
            }
            let narrow_off = i * sew_bytes;
            let nr = narrow_off / vlenb;
            let no = narrow_off % vlenb;
            let vs1_chunk = state.reg.vr.raw_read(vs1 + nr);
            let vs2_chunk = state.reg.vr.raw_read(vs2 + nr);
            let s1_zext: u64 = match sew_bytes {
                1 => vs1_chunk[no] as u64,
                2 => u16::from_le_bytes(vs1_chunk[no..no + 2].try_into().unwrap()) as u64,
                _ => u32::from_le_bytes(vs1_chunk[no..no + 4].try_into().unwrap()) as u64,
            };
            let s2_zext: u64 = match sew_bytes {
                1 => vs2_chunk[no] as u64,
                2 => u16::from_le_bytes(vs2_chunk[no..no + 2].try_into().unwrap()) as u64,
                _ => u32::from_le_bytes(vs2_chunk[no..no + 4].try_into().unwrap()) as u64,
            };
            let res = s1_zext.wrapping_mul(s2_zext);
            let wide_off = i * dst_bytes_per_elem;
            let wo = wide_off % vlenb;
            match dst_bytes_per_elem {
                2 => dst_chunk[wo..wo + 2].copy_from_slice(&(res as u16).to_le_bytes()),
                4 => dst_chunk[wo..wo + 4].copy_from_slice(&(res as u32).to_le_bytes()),
                8 => dst_chunk[wo..wo + 8].copy_from_slice(&res.to_le_bytes()),
                _ => {}
            }
        }
        state.reg.vr.raw_write(vd + r, &dst_chunk);
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

fn vector_zext_vf4<P, C>(ctx: &mut C, decoded: &DecodedInst) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let state = ctx.state_mut();
    let vd = decoded.rd as usize;
    let vs2 = decoded.rs2 as usize;
    let vm = decoded.imm != 0;

    let vl = state.reg.csr.vector.vl();
    let vtype = state.reg.csr.vector.vtype();
    let vlmul = vtype & 0x7;
    let vsew = (vtype >> 3) & 0x7;
    let sew_bytes = 1 << (vsew & 0x3);
    let src_sew_bytes = sew_bytes / 4;
    if src_sew_bytes == 0 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
    let nf = nf_from_vlmul(vlmul)
        .min(32_usize.saturating_sub(vd))
        .min(32_usize.saturating_sub(vs2));
    let total_elems = (nf * vlenb) / sew_bytes;
    let n = vl.min(total_elems as u32) as usize;

    let v0 = state.reg.vr.raw_read(0).to_vec();

    for r in 0..nf {
        let mut dst_chunk = state.reg.vr.raw_read(vd + r).to_vec();
        let start_elem = (r * vlenb) / sew_bytes;
        let end_elem = ((r + 1) * vlenb) / sew_bytes;
        let loop_end = end_elem.min(n);

        for i in start_elem..loop_end {
            let active = vm || ((v0[i / 8] >> (i % 8)) & 1 != 0);
            if !active {
                continue;
            }
            let src_byte_off = i * src_sew_bytes;
            let src_reg = src_byte_off / vlenb;
            let src_off = src_byte_off % vlenb;
            let src_chunk = state.reg.vr.raw_read(vs2 + src_reg);
            let mut val = [0u8; 8];
            match (src_sew_bytes, sew_bytes) {
                (1, 4) => {
                    let b = src_chunk[src_off];
                    val[..4].copy_from_slice(&(b as u32).to_le_bytes());
                }
                (1, 8) => {
                    let b = src_chunk[src_off];
                    val[..8].copy_from_slice(&(b as u64).to_le_bytes());
                }
                (2, 4) => {
                    let w = u16::from_le_bytes(src_chunk[src_off..src_off + 2].try_into().unwrap());
                    val[..4].copy_from_slice(&(w as u32).to_le_bytes());
                }
                (2, 8) => {
                    let w = u16::from_le_bytes(src_chunk[src_off..src_off + 2].try_into().unwrap());
                    val[..8].copy_from_slice(&(w as u64).to_le_bytes());
                }
                _ => continue,
            }
            let dst_off = (i * sew_bytes) % vlenb;
            dst_chunk[dst_off..dst_off + sew_bytes].copy_from_slice(&val[..sew_bytes]);
        }
        state.reg.vr.raw_write(vd + r, &dst_chunk);
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    op: OpMvvInst,
) -> Result<(), remu_state::StateError> {
    match op {
        OpMvvInst::Vredsum_vs => vector_redsum_vs::<P, C>(ctx, decoded),
        OpMvvInst::Vid_v => vector_element_loop(
            ctx,
            decoded.rd as usize,
            None,
            VectorElementLoopMode::Unmasked,
            |idx, _, _, _mask, _dst| idx as u64,
        ),
        OpMvvInst::Vmv_x_s => {
            let state = ctx.state_mut();
            let vtype = state.reg.csr.vector.vtype();
            let vsew = (vtype >> 3) & 0x7;
            let sew_bytes = 1 << (vsew & 0x3);
            let vs2_chunk = state.reg.vr.raw_read(decoded.rs2 as usize);
            let res = match sew_bytes {
                1 => (vs2_chunk[0] as i8 as i64) as u64,
                2 => (i16::from_le_bytes(vs2_chunk[0..2].try_into().unwrap()) as i64) as u64,
                4 => (i32::from_le_bytes(vs2_chunk[0..4].try_into().unwrap()) as i64) as u64,
                8 => i64::from_le_bytes(vs2_chunk[0..8].try_into().unwrap()) as u64,
                _ => 0,
            };
            state.reg.gpr.raw_write(decoded.rd.into(), res as u32);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
        OpMvvInst::Vfirst_m => {
            let state = ctx.state_mut();
            let vl = state.reg.csr.vector.vl();
            let vs2_chunk = state.reg.vr.raw_read(decoded.rs2 as usize);
            let mut pos = !0u32;
            for i in 0..vl {
                let byte_idx = (i as usize) / 8;
                let bit_idx = i % 8;
                if byte_idx < vs2_chunk.len() && (vs2_chunk[byte_idx] >> bit_idx) & 1 != 0 {
                    pos = i;
                    break;
                }
            }
            state.reg.gpr.raw_write(decoded.rd.into(), pos);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
        OpMvvInst::Vsext_vf4 => vector_sext_vf4::<P, C>(ctx, decoded),
        OpMvvInst::Vzext_vf4 => vector_zext_vf4::<P, C>(ctx, decoded),
        OpMvvInst::Vmor_mm => {
            let state = ctx.state_mut();
            let vl = state.reg.csr.vector.vl() as usize;
            let vs1_buf = state.reg.vr.raw_read(decoded.rs1 as usize);
            let vs2_buf = state.reg.vr.raw_read(decoded.rs2 as usize);
            let mut vd_buf = state.reg.vr.raw_read(decoded.rd as usize).to_vec();
            for i in 0..vl {
                let byte_idx = i / 8;
                let bit_idx = i % 8;
                let bit1 = (vs1_buf[byte_idx] >> bit_idx) & 1;
                let bit2 = (vs2_buf[byte_idx] >> bit_idx) & 1;
                if (bit1 | bit2) != 0 {
                    vd_buf[byte_idx] |= 1 << bit_idx;
                } else {
                    vd_buf[byte_idx] &= !(1 << bit_idx);
                }
            }
            state.reg.vr.raw_write(decoded.rd as usize, &vd_buf);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
        OpMvvInst::Vmacc_vv => {
            let vm = decoded.imm != 0;
            let mode = if vm {
                VectorElementLoopMode::Unmasked
            } else {
                VectorElementLoopMode::Masked
            };
            vector_element_loop_vv(
                ctx,
                decoded.rd as usize,
                decoded.rs1 as usize,
                decoded.rs2 as usize,
                mode,
                |_, sew_bytes, src1, src2, mask, dst| {
                    if mask {
                        match sew_bytes {
                            1 => {
                                let prod = (src1 as i8 as i16).wrapping_mul(src2 as i8 as i16);
                                (prod.wrapping_add(dst as i8 as i16) as i8 as u8) as u64
                            }
                            2 => {
                                let prod = (src1 as i16 as i32).wrapping_mul(src2 as i16 as i32);
                                (prod.wrapping_add(dst as i16 as i32) as i16 as u16) as u64
                            }
                            4 => {
                                let prod = (src1 as i32 as i64).wrapping_mul(src2 as i32 as i64);
                                (prod.wrapping_add(dst as i32 as i64) as i32 as u32) as u64
                            }
                            8 => {
                                let prod = (src1 as i64).wrapping_mul(src2 as i64);
                                prod.wrapping_add(dst as i64) as u64
                            }
                            _ => dst,
                        }
                    } else {
                        dst
                    }
                },
            )
        }
        OpMvvInst::Vwmacc_vv => vector_wmacc_vv::<P, C>(ctx, decoded),
        OpMvvInst::Vwmulu_vv => vector_wmulu_vv::<P, C>(ctx, decoded),
    }
}
