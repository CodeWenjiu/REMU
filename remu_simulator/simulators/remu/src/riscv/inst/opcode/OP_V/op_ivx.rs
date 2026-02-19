//! funct3 = 0b100: OP-IVX

use remu_types::isa::{
    extension_v::VExtensionConfig,
    reg::{RegAccess, VectorCsrState, VrState},
    RvIsa,
};

use crate::riscv::inst::{DecodedInst, opcode::OP_V::OpIvxInst};

use super::op_mvv::{nf_from_vlmul, vector_element_loop_masked};

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
    op: OpIvxInst,
) -> Result<(), remu_state::StateError> {
    match op {
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
            vector_mask_cmp_vx::<P, C>(
                ctx,
                decoded.rd as usize,
                decoded.rs2 as usize,
                decoded.rs1,
            )
        }
    }
}
