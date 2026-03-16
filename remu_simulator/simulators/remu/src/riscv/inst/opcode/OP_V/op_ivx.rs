//! funct3 = 0b100: OP-IVX

use remu_types::isa::reg::{RegAccess, VrState};

use crate::riscv::inst::{DecodedInst, opcode::OP_V::OpIvxInst};

use super::{VContext, utils::{vector_element_loop, VectorElementLoopMode}};

fn vector_mask_cmp_vx<P, C, F>(
    ctx: &mut C,
    decoded: &DecodedInst,
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
    let rs1 = decoded.rs1;
    let vm = decoded.imm != 0;

    let nf = vctx.nf.min(32_usize.saturating_sub(vs2));
    let total_elems = (nf * vctx.vlenb) / vctx.sew_bytes;
    let n = vctx.vl.min(total_elems as u32) as usize;
    let scalar = state.reg.gpr.raw_read(rs1.into());

    let v0 = state.reg.vr.raw_read(0).to_vec();
    let mut vd_buf = state.reg.vr.raw_read(vd).to_vec();

    let rs1_sext = match vctx.sew_bytes {
        1 => (scalar as i8) as i64,
        2 => (scalar as i16) as i64,
        4 => (scalar as i32) as i64,
        8 => scalar as i64,
        _ => 0,
    };

    for i in 0..n {
        let active = vm || super::mask_bit(&v0, i);
        let result_bit = if active {
            let (_, reg_i, off) = vctx.elem_layout(i);
            let chunk = state.reg.vr.raw_read(vs2 + reg_i);
            let vs2_val = vctx.sew.read_i(chunk, off);
            cmp_op(vs2_val, rs1_sext)
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

pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    op: OpIvxInst,
) -> Result<(), remu_state::StateError> {
    match op {
        OpIvxInst::Vmerge_vxm => {
            let scalar = ctx.state_mut().reg.gpr.raw_read(decoded.rs1.into());
            let vm = decoded.imm != 0;
            let mode = if vm {
                VectorElementLoopMode::Unmasked
            } else {
                VectorElementLoopMode::Masked
            };
            vector_element_loop(
                ctx,
                decoded.rd as usize,
                Some(decoded.rs2 as usize),
                mode,
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
        OpIvxInst::Vadd_vx => {
            let scalar_val = ctx.state_mut().reg.gpr.raw_read(decoded.rs1.into()) as u64;
            let vm = decoded.imm != 0;
            let mode = if vm {
                VectorElementLoopMode::Unmasked
            } else {
                VectorElementLoopMode::Masked
            };
            vector_element_loop(
                ctx,
                decoded.rd as usize,
                Some(decoded.rs2 as usize),
                mode,
                |_, sew_bytes, src, mask, dst| {
                    if mask {
                        let v = src.unwrap_or(0);
                        match sew_bytes {
                            1 => (v as u8).wrapping_add(scalar_val as u8) as u64,
                            2 => (v as u16).wrapping_add(scalar_val as u16) as u64,
                            4 => (v as u32).wrapping_add(scalar_val as u32) as u64,
                            8 => (v as u64).wrapping_add(scalar_val),
                            _ => 0,
                        }
                    } else {
                        dst
                    }
                },
            )
        }
        OpIvxInst::Vand_vx => {
            let scalar_val = ctx.state_mut().reg.gpr.raw_read(decoded.rs1.into()) as u64;
            let vm = decoded.imm != 0;
            let mode = if vm {
                VectorElementLoopMode::Unmasked
            } else {
                VectorElementLoopMode::Masked
            };
            vector_element_loop(
                ctx,
                decoded.rd as usize,
                Some(decoded.rs2 as usize),
                mode,
                |_, sew_bytes, src, mask, dst| {
                    if mask {
                        let val = src.unwrap_or(0);
                        match sew_bytes {
                            1 => (val as u8 & scalar_val as u8) as u64,
                            2 => (val as u16 & scalar_val as u16) as u64,
                            4 => (val as u32 & scalar_val as u32) as u64,
                            8 => val & scalar_val,
                            _ => 0,
                        }
                    } else {
                        dst
                    }
                },
            )
        }
        OpIvxInst::Vmslt_vx => {
            vector_mask_cmp_vx::<P, C, _>(ctx, decoded, |a, b| a < b)
        }
        OpIvxInst::Vmseq_vx => {
            vector_mask_cmp_vx::<P, C, _>(ctx, decoded, |a, b| a == b)
        }
    }
}
