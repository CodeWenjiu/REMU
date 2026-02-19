use remu_types::isa::{
    RvIsa,
    extension_v::VExtensionConfig,
    reg::{RegAccess, VectorCsrState, VrState},
};

use crate::riscv::inst::{
    DecodedInst, Inst,
    opcode::OP_V::{OpCfgInst, OpIviInst, OpIvxInst, OpMvvInst, VInst},
};

#[inline(always)]
fn zimm_to_vtype(zimm: u32) -> u32 {
    zimm & 0xFF
}

#[inline(always)]
fn nf_from_vlmul(vlmul: u32) -> usize {
    match vlmul & 0x7 {
        0 => 1,
        1 => 2,
        2 => 4,
        3 => 8,
        _ => 1,
    }
}

#[inline(always)]
fn calculate_vlmax(vlenb: u32, vtype: u32) -> u32 {
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
        0..=3 => -(vsew as i8), // 0->0, 1->-1, 2->-2, 3->-3
        _ => return 0,          // Reserved
    };

    let total_shift = lmul_shift + sew_shift;

    if total_shift >= 0 {
        vlenb << total_shift
    } else {
        vlenb >> (-total_shift)
    }
}

/// Generic vector element loop helper.
/// Handles: V configuration (vl, vtype, sew), multi-register grouping (nf), loop bounds,
/// reading from rs2 (optional), and writing to rd.
/// `op` closure receives: (element_index, sew_bytes, optional_rs2_value).
/// `op` must return the result as u64 (bits), which will be truncated to SEW.
#[inline(always)]
fn vector_element_loop<P, C, F>(
    ctx: &mut C,
    rd: usize,
    rs2: Option<usize>,
    mut op: F,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
    F: FnMut(u32, usize, Option<u64>) -> u64,
{
    let state = ctx.state_mut();
    let vl = state.reg.csr.vector.vl();
    let vtype = state.reg.csr.vector.vtype();
    let vlmul = vtype & 0x7;
    let vsew = (vtype >> 3) & 0x7;
    let sew_bytes = 1 << (vsew & 0x3); // 0->1, 1->2, 2->4, 3->8. Ignore reserved 4-7.

    let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
    let nf_max = nf_from_vlmul(vlmul);

    // Clamp nf to avoid register file overflow (spec: illegal if > 31)
    let mut nf = nf_max.min(32_usize.saturating_sub(rd));
    if let Some(r2) = rs2 {
        nf = nf.min(32_usize.saturating_sub(r2));
    }

    let total_elems = (nf * vlenb) / sew_bytes;
    let n = vl.min(total_elems as u32);

    for r in 0..nf {
        // Read source register chunk if needed (copy to avoid borrow conflict)
        let src_chunk = rs2.map(|reg| state.reg.vr.raw_read(reg + r).to_vec());
        let mut dst_chunk = vec![0u8; vlenb];

        // Process elements belonging to this register
        // Element i is in register r if (i * sew) / vlenb == r
        let start_elem = (r * vlenb) / sew_bytes;
        let end_elem = ((r + 1) * vlenb) / sew_bytes;
        // Intersect with [0, n)
        let loop_start = (start_elem as u32).min(n);
        let loop_end = (end_elem as u32).min(n);

        for i in loop_start..loop_end {
            let off = ((i as usize) * sew_bytes) % vlenb;

            // Extract source operand
            let src_val = src_chunk.as_ref().map(|chunk| match sew_bytes {
                1 => chunk[off] as u64,
                2 => u16::from_le_bytes(chunk[off..off + 2].try_into().unwrap()) as u64,
                4 => u32::from_le_bytes(chunk[off..off + 4].try_into().unwrap()) as u64,
                8 => u64::from_le_bytes(chunk[off..off + 8].try_into().unwrap()),
                _ => 0,
            });

            let res = op(i, sew_bytes, src_val);

            // Write result
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

/// Like `vector_element_loop` but reads mask from v0 (1 bit per element, LSB-packed)
/// and passes (element_index, sew_bytes, optional_rs2_value, mask_bit, dst_current) to the closure.
/// When mask_bit is false, op should return dst_current to leave vd[i] unchanged.
#[inline(always)]
fn vector_element_loop_masked<P, C, F>(
    ctx: &mut C,
    rd: usize,
    rs2: Option<usize>,
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

    let v0 = state.reg.vr.raw_read(0).to_vec();

    for r in 0..nf {
        let src_chunk = rs2.map(|reg| state.reg.vr.raw_read(reg + r).to_vec());
        let mut dst_chunk = state.reg.vr.raw_read(rd + r).to_vec();

        let start_elem = (r * vlenb) / sew_bytes;
        let end_elem = ((r + 1) * vlenb) / sew_bytes;
        let loop_start = (start_elem as u32).min(n);
        let loop_end = (end_elem as u32).min(n);

        for i in loop_start..loop_end {
            let off = ((i as usize) * sew_bytes) % vlenb;
            let mask_bit = (v0[(i as usize) / 8] >> (i % 8)) & 1 != 0;

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

/// vsext.vf4: vd[i] = sign_extend(vs2[i] from SEW/4 to SEW). EEW = SEW/4.
#[inline(always)]
fn vector_sext_vf4<P, C>(ctx: &mut C, vd: usize, vs2: usize) -> Result<(), remu_state::StateError>
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

    for r in 0..nf {
        let mut dst_chunk = state.reg.vr.raw_read(vd + r).to_vec();
        let start_elem = (r * vlenb) / sew_bytes;
        let end_elem = ((r + 1) * vlenb) / sew_bytes;
        let loop_end = end_elem.min(n);

        for i in start_elem..loop_end {
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

/// vmseq.vi: write mask to vd (1 bit per element). vd[i] = (vs2[i] == simm5_sext) ? 1 : 0.
/// vd is a single register; tail bits (vl..VLEN) are left unchanged.
#[inline(always)]
fn vector_mask_cmp_vi<P, C>(
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

/// vredsum.vs: vd[0] = vs1[0] + sum(vs2[0..vl]) (signed, SEW).
#[inline(always)]
fn vector_redsum_vs<P, C>(
    ctx: &mut C,
    vd: usize,
    vs1: usize,
    vs2: usize,
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

/// vslidedown.vi: vd[i] = vs2[i+sh] if (i+sh)<vl else 0; tail zeroed.
#[inline(always)]
fn vector_slidedown_vi<P, C>(
    ctx: &mut C,
    vd: usize,
    vs2: usize,
    uimm5: u32,
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
    let nf = nf_from_vlmul(vlmul)
        .min(32_usize.saturating_sub(vd))
        .min(32_usize.saturating_sub(vs2));
    let total_elems = (nf * vlenb) / sew_bytes;
    let n = vl.min(total_elems as u32) as usize;
    let sh = uimm5 as usize;

    for r in 0..nf {
        let mut dst_chunk = vec![0u8; vlenb];
        let start_elem = (r * vlenb) / sew_bytes;
        let end_elem = ((r + 1) * vlenb) / sew_bytes;
        let loop_end = end_elem.min(n);

        for i in start_elem..loop_end {
            let src_idx = i.wrapping_add(sh);
            let val = if src_idx < vl as usize {
                let byte_off = src_idx * sew_bytes;
                let reg_i = byte_off / vlenb;
                let off = byte_off % vlenb;
                let chunk = state.reg.vr.raw_read(vs2 + reg_i);
                match sew_bytes {
                    1 => chunk[off] as u64,
                    2 => u16::from_le_bytes(chunk[off..off + 2].try_into().unwrap()) as u64,
                    4 => u32::from_le_bytes(chunk[off..off + 4].try_into().unwrap()) as u64,
                    8 => u64::from_le_bytes(chunk[off..off + 8].try_into().unwrap()),
                    _ => 0,
                }
            } else {
                0
            };
            let dst_off = (i * sew_bytes) % vlenb;
            match sew_bytes {
                1 => dst_chunk[dst_off] = val as u8,
                2 => dst_chunk[dst_off..dst_off + 2].copy_from_slice(&(val as u16).to_le_bytes()),
                4 => dst_chunk[dst_off..dst_off + 4].copy_from_slice(&(val as u32).to_le_bytes()),
                8 => dst_chunk[dst_off..dst_off + 8].copy_from_slice(&val.to_le_bytes()),
                _ => {}
            }
        }
        state.reg.vr.raw_write(vd + r, &dst_chunk);
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// vmslt.vx: write mask to vd. vd[i] = (vs2[i] < rs1_sext) ? 1 : 0 (signed).
#[inline(always)]
fn vector_mask_cmp_vx<P, C>(
    ctx: &mut C,
    vd: usize,
    vs2: usize,
    rs1: u8,
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
    let scalar = state.reg.gpr.raw_read(rs1.into());

    let mut vd_buf = state.reg.vr.raw_read(vd).to_vec();

    for i in 0..n {
        let byte_offset = (i as usize) * sew_bytes;
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
        let rs1_sext = match sew_bytes {
            1 => (scalar as i8) as i64,
            2 => (scalar as i16) as i64,
            4 => (scalar as i32) as i64,
            8 => scalar as i64,
            _ => 0,
        };
        let bit = (vs2_val < rs1_sext) as u8;
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

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    let v = match decoded.inst {
        Inst::V(v) => v,
        _ => return crate::riscv::inst::opcode::UNKNOWN::execute::<P, C>(ctx, decoded),
    };

    match v {
        VInst::OpCfg(op) => match op {
            OpCfgInst::Vsetivli => {
                let zimm = (decoded.imm >> 5) & 0x3FF;
                let vtype = zimm_to_vtype(zimm);
                let uimm = decoded.imm & 0x1F;
                let rd = decoded.rd;
                let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB;
                let vlmax = calculate_vlmax(vlenb, vtype);
                let vl = uimm.min(vlmax);
                let state = ctx.state_mut();
                state.reg.csr.vector.set_vtype(vtype);
                state.reg.csr.vector.set_vl(vl);
                state.reg.gpr.raw_write(rd.into(), vl);
                *state.reg.pc = state.reg.pc.wrapping_add(4);
                Ok(())
            }
            OpCfgInst::Vsetvli => {
                let zimm = (decoded.imm >> 5) & 0x3FF;
                let vtype = zimm_to_vtype(zimm);
                let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB;
                let vlmax = calculate_vlmax(vlenb, vtype);
                let rd = decoded.rd;
                let rs1 = decoded.rs1;
                let state = ctx.state_mut();
                state.reg.csr.vector.set_vtype(vtype);
                *state.reg.pc = state.reg.pc.wrapping_add(4);
                if rs1 == 0 && rd == 0 {
                    return Ok(());
                }
                let avl = if rs1 == 0 { u32::MAX } else { state.reg.gpr.raw_read(rs1.into()) };
                let vl = avl.min(vlmax);
                state.reg.csr.vector.set_vl(vl);
                state.reg.gpr.raw_write(rd.into(), vl);
                Ok(())
            }
        },
        VInst::OpMvv(op) => match op {
            OpMvvInst::Vredsum_vs => vector_redsum_vs::<P, C>(
                ctx,
                decoded.rd as usize,
                decoded.rs1 as usize,
                decoded.rs2 as usize,
            ),
            OpMvvInst::Vid_v => {
                vector_element_loop(ctx, decoded.rd as usize, None, |idx, _, _| idx as u64)
            }
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
            OpMvvInst::Vmerge_vim => {
                let simm5 = decoded.imm as i32;
                let vm = decoded.rs1 != 0;
                if vm {
                    vector_element_loop(ctx, decoded.rd as usize, None, |_, sew, _| match sew {
                        1 => (simm5 as i8) as u8 as u64,
                        2 => (simm5 as i16) as u16 as u64,
                        4 => (simm5 as u32) as u64,
                        8 => simm5 as i64 as u64,
                        _ => 0,
                    })
                } else {
                    vector_element_loop_masked(
                        ctx,
                        decoded.rd as usize,
                        Some(decoded.rs2 as usize),
                        |_, sew, src, mask, _dst| {
                            if mask {
                                match sew {
                                    1 => (simm5 as i8) as u8 as u64,
                                    2 => (simm5 as i16) as u16 as u64,
                                    4 => (simm5 as u32) as u64,
                                    8 => simm5 as i64 as u64,
                                    _ => 0,
                                }
                            } else {
                                src.unwrap_or(0)
                            }
                        },
                    )
                }
            }
            OpMvvInst::Vmseq_vi => {
                vector_mask_cmp_vi::<P, C>(ctx, decoded.rd as usize, decoded.rs2 as usize, decoded.imm)
            }
            OpMvvInst::Vsext_vf4 => {
                vector_sext_vf4::<P, C>(ctx, decoded.rd as usize, decoded.rs2 as usize)
            }
        },
        VInst::OpIvi(op) => match op {
            OpIviInst::Vmerge_vim => {
                let simm5 = decoded.imm as i32;
                let vm = decoded.rs1 != 0;
                if vm {
                    vector_element_loop(ctx, decoded.rd as usize, None, |_, sew, _| match sew {
                        1 => (simm5 as i8) as u8 as u64,
                        2 => (simm5 as i16) as u16 as u64,
                        4 => (simm5 as u32) as u64,
                        8 => simm5 as i64 as u64,
                        _ => 0,
                    })
                } else {
                    vector_element_loop_masked(
                        ctx,
                        decoded.rd as usize,
                        Some(decoded.rs2 as usize),
                        |_, sew, src, mask, _dst| {
                            if mask {
                                match sew {
                                    1 => (simm5 as i8) as u8 as u64,
                                    2 => (simm5 as i16) as u16 as u64,
                                    4 => (simm5 as u32) as u64,
                                    8 => simm5 as i64 as u64,
                                    _ => 0,
                                }
                            } else {
                                src.unwrap_or(0)
                            }
                        },
                    )
                }
            }
            OpIviInst::Vmseq_vi => {
                vector_mask_cmp_vi::<P, C>(ctx, decoded.rd as usize, decoded.rs2 as usize, decoded.imm)
            }
            OpIviInst::Vmv1r_v => {
                let state = ctx.state_mut();
                let vs2 = decoded.rs2 as usize;
                let vd = decoded.rd as usize;
                let data = state.reg.vr.raw_read(vs2).to_vec();
                state.reg.vr.raw_write(vd, &data);
                *state.reg.pc = state.reg.pc.wrapping_add(4);
                Ok(())
            }
            OpIviInst::Vrsub_vi => {
                let simm5 = decoded.imm as i32;
                vector_element_loop(
                    ctx,
                    decoded.rd as usize,
                    Some(decoded.rs2 as usize),
                    |_, sew, src| {
                        let v = src.unwrap_or(0);
                        match sew {
                            1 => (simm5 as i8).wrapping_sub(v as i8) as u8 as u64,
                            2 => (simm5 as i16).wrapping_sub(v as i16) as u16 as u64,
                            4 => simm5.wrapping_sub(v as i32) as u32 as u64,
                            8 => (simm5 as i64).wrapping_sub(v as i64) as u64,
                            _ => 0,
                        }
                    },
                )
            }
            OpIviInst::Vadd_vi => {
                let simm5 = decoded.imm as i32;
                let vm = decoded.rs1 != 0;
                if vm {
                    vector_element_loop(
                        ctx,
                        decoded.rd as usize,
                        Some(decoded.rs2 as usize),
                        |_, sew, src| {
                            let v = src.unwrap_or(0);
                            match sew {
                                1 => (simm5 as i8).wrapping_add(v as i8) as u8 as u64,
                                2 => (simm5 as i16).wrapping_add(v as i16) as u16 as u64,
                                4 => simm5.wrapping_add(v as i32) as u32 as u64,
                                8 => (simm5 as i64).wrapping_add(v as i64) as u64,
                                _ => 0,
                            }
                        },
                    )
                } else {
                    vector_element_loop_masked(
                        ctx,
                        decoded.rd as usize,
                        Some(decoded.rs2 as usize),
                        |_, sew, src, mask, dst| {
                            if mask {
                                let v = src.unwrap_or(0);
                                match sew {
                                    1 => (simm5 as i8).wrapping_add(v as i8) as u8 as u64,
                                    2 => (simm5 as i16).wrapping_add(v as i16) as u16 as u64,
                                    4 => simm5.wrapping_add(v as i32) as u32 as u64,
                                    8 => (simm5 as i64).wrapping_add(v as i64) as u64,
                                    _ => 0,
                                }
                            } else {
                                dst
                            }
                        },
                    )
                }
            }
            OpIviInst::Vslidedown_vi => {
                vector_slidedown_vi::<P, C>(ctx, decoded.rd as usize, decoded.rs2 as usize, decoded.imm)
            }
        },
        VInst::OpIvx(op) => match op {
            OpIvxInst::Vmerge_vxm => {
                let scalar = ctx.state_mut().reg.gpr.raw_read(decoded.rs1.into());
                vector_element_loop_masked(
                    ctx,
                    decoded.rd as usize,
                    Some(decoded.rs2 as usize),
                    |_, sew, src, mask, _dst| {
                        if mask {
                            match sew {
                                1 => (scalar as i32 as i8) as u8 as u64,
                                2 => (scalar as i32 as i16) as u16 as u64,
                                4 => (scalar as i32 as u32) as u64,
                                8 => (scalar as i32 as i64) as u64,
                                _ => 0,
                            }
                        } else {
                            src.unwrap_or(0)
                        }
                    },
                )
            }
            OpIvxInst::Vmslt_vx => {
                vector_mask_cmp_vx::<P, C>(ctx, decoded.rd as usize, decoded.rs2 as usize, decoded.rs1)
            }
            OpIvxInst::Vmv_s_x => {
                let state = ctx.state_mut();
                let vl = state.reg.csr.vector.vl();
                if vl == 0 {
                    *state.reg.pc = state.reg.pc.wrapping_add(4);
                    return Ok(());
                }
                let vtype = state.reg.csr.vector.vtype();
                let vsew = (vtype >> 3) & 0x7;
                let sew_bytes = 1 << (vsew & 0x3);
                let scalar = state.reg.gpr.raw_read(decoded.rs1.into());
                let mut chunk = state.reg.vr.raw_read(decoded.rd as usize).to_vec();
                match sew_bytes {
                    1 => chunk[0] = (scalar & 0xFF) as u8,
                    2 => chunk[0..2].copy_from_slice(&(scalar & 0xFFFF).to_le_bytes()),
                    4 => chunk[0..4].copy_from_slice(&(scalar & 0xFFFF_FFFF).to_le_bytes()),
                    8 => chunk[0..8].copy_from_slice(&(scalar as u64).to_le_bytes()),
                    _ => {}
                }
                state.reg.vr.raw_write(decoded.rd as usize, &chunk);
                *state.reg.pc = state.reg.pc.wrapping_add(4);
                Ok(())
            }
        },
    }
}
