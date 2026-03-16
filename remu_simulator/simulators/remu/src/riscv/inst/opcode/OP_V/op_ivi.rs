//! funct3 = 0b011: OP-IVI

use remu_types::isa::reg::VrState;

use crate::riscv::inst::{DecodedInst, opcode::OP_V::OpIviInst};

use super::{
    VContext,
    utils::{vector_element_loop, vector_mask_cmp_vi, VectorElementLoopMode},
};

fn vector_slidedown_vi<P, C>(
    ctx: &mut C,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let vctx = VContext::from_state::<P, C>(ctx);
    let state = ctx.state_mut();
    let uimm5 = (decoded.imm & 0x1F) as usize;
    let vm = (decoded.imm >> 8) != 0;

    let vd = decoded.rd as usize;
    let vs2 = decoded.rs2 as usize;

    let vlmax = vctx.vlmax as usize;
    let n = vctx.vl.min(vlmax as u32) as usize;
    let nf = vctx.nf
        .min(32_usize.saturating_sub(vd))
        .min(32_usize.saturating_sub(vs2))
        .max(1);

    let vs2_buf: Vec<Vec<u8>> = (0..nf).map(|r| state.reg.vr.raw_read(vs2 + r).to_vec()).collect();
    let mut vd_buf: Vec<Vec<u8>> = (0..nf).map(|r| state.reg.vr.raw_read(vd + r).to_vec()).collect();
    let v0 = state.reg.vr.raw_read(0).to_vec();

    for i in 0..n {
        let active = vm || super::mask_bit(&v0, i);
        if !active {
            continue;
        }
        let src_idx = i + uimm5;
        let val = if src_idx >= vlmax {
            0u64
        } else {
            let (_, reg_i, off) = vctx.elem_layout(src_idx);
            vctx.sew.read_u(&vs2_buf[reg_i], off)
        };
        let (_, dst_reg, dst_off) = vctx.elem_layout(i);
        vctx.sew.write(&mut vd_buf[dst_reg], dst_off, val);
    }

    for r in 0..nf {
        state.reg.vr.raw_write(vd + r, &vd_buf[r]);
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    op: OpIviInst,
) -> Result<(), remu_state::StateError> {
    match op {
        OpIviInst::Vmerge_vim => {
            let simm5 = ((decoded.imm << 27) as i32) >> 27;
            let vm = decoded.rs1 != 0;
            let mode = if vm {
                VectorElementLoopMode::Unmasked
            } else {
                VectorElementLoopMode::Masked
            };
            vector_element_loop(
                ctx,
                decoded.rd as usize,
                if vm { None } else { Some(decoded.rs2 as usize) },
                mode,
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
        OpIviInst::Vmseq_vi => vector_mask_cmp_vi::<P, C, _>(ctx, decoded, |a, b| a == b),
        OpIviInst::Vmsne_vi => vector_mask_cmp_vi::<P, C, _>(ctx, decoded, |a, b| a != b),
        OpIviInst::VmvNr_v => {
            let nr = (decoded.imm + 1) as usize; // vmv1r/2r/4r/8r: simm5=0,1,3,7 -> nr=1,2,4,8
            if nr != 1 && nr != 2 && nr != 4 && nr != 8 {
                return Err(remu_state::StateError::BusError(Box::new(
                    remu_state::bus::BusError::unmapped(0),
                )));
            }
            let vd_base = decoded.rd as usize;
            let vs2_base = decoded.rs2 as usize;
            if vd_base % nr != 0 || vs2_base % nr != 0 {
                return Err(remu_state::StateError::BusError(Box::new(
                    remu_state::bus::BusError::unmapped(0),
                )));
            }
            if vd_base + nr > 32 || vs2_base + nr > 32 {
                return Err(remu_state::StateError::BusError(Box::new(
                    remu_state::bus::BusError::unmapped(0),
                )));
            }
            if vd_base != vs2_base && vd_base < vs2_base + nr && vs2_base < vd_base + nr {
                return Err(remu_state::StateError::BusError(Box::new(
                    remu_state::bus::BusError::unmapped(0),
                )));
            }
            let state = ctx.state_mut();
            for i in 0..nr {
                let data = state.reg.vr.raw_read(vs2_base + i).to_vec();
                state.reg.vr.raw_write(vd_base + i, &data);
            }
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
        OpIviInst::Vrsub_vi => {
            let simm5 = ((decoded.imm << 27) as i32) >> 27;
            let vm = decoded.rs1 != 0;
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
                |_, sew, src, mask, dst| {
                    if mask {
                        let v = src.unwrap_or(0);
                        match sew {
                            1 => (simm5 as i8).wrapping_sub(v as i8) as u8 as u64,
                            2 => (simm5 as i16).wrapping_sub(v as i16) as u16 as u64,
                            4 => simm5.wrapping_sub(v as i32) as u32 as u64,
                            8 => (simm5 as i64).wrapping_sub(v as i64) as u64,
                            _ => 0,
                        }
                    } else {
                        dst
                    }
                },
            )
        }
        OpIviInst::Vadd_vi => {
            let simm5 = ((decoded.imm << 27) as i32) >> 27;
            let vm = decoded.rs1 != 0;
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
        OpIviInst::Vslidedown_vi => vector_slidedown_vi::<P, C>(ctx, decoded),
        OpIviInst::Vsll_vi => {
            let uimm5 = decoded.imm & 0x1F;
            let vm = (decoded.imm >> 8) != 0;
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
                        let bit_width = (sew_bytes * 8) as u32;
                        let shift_mask = bit_width - 1;
                        let shamt = uimm5 & shift_mask;
                        match sew_bytes {
                            1 => ((v as u8).wrapping_shl(shamt)) as u64,
                            2 => ((v as u16).wrapping_shl(shamt)) as u64,
                            4 => ((v as u32).wrapping_shl(shamt)) as u64,
                            8 => (v as u64).wrapping_shl(shamt),
                            _ => 0,
                        }
                    } else {
                        dst
                    }
                },
            )
        }
        OpIviInst::Vsrl_vi => {
            let uimm5 = decoded.imm & 0x1F;
            let vm = (decoded.imm >> 8) != 0;
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
                        let bit_width = (sew_bytes * 8) as u32;
                        let shift_mask = bit_width - 1;
                        let shamt = uimm5 & shift_mask;
                        match sew_bytes {
                            1 => ((v as u8).wrapping_shr(shamt)) as u64,
                            2 => ((v as u16).wrapping_shr(shamt)) as u64,
                            4 => ((v as u32).wrapping_shr(shamt)) as u64,
                            8 => (v as u64).wrapping_shr(shamt),
                            _ => 0,
                        }
                    } else {
                        dst
                    }
                },
            )
        }
        OpIviInst::Vand_vi => {
            let simm5 = decoded.imm as i32;
            let vm = decoded.rs1 != 0;
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
                            1 => (val as u8 & (simm5 as i8 as u8)) as u64,
                            2 => (val as u16 & (simm5 as i16 as u16)) as u64,
                            4 => (val as u32 & simm5 as u32) as u64,
                            8 => val & (simm5 as i64 as u64),
                            _ => 0,
                        }
                    } else {
                        dst
                    }
                },
            )
        }
    }
}
