//! funct3 = 0b011: OP-IVI

use remu_types::isa::reg::VrState;

use crate::riscv::inst::{DecodedInst, opcode::OP_V::OpIviInst};

use super::{
    loop_ops::{binop_add_vi, binop_and_vi, binop_shl_vi, binop_shr_vi, binop_sub_vi, merge_scalar_vi, mode_from_vm},
    utils::{vector_element_loop, vector_mask_cmp, vector_slide, vector_slide_up},
};

pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    op: OpIviInst,
) -> Result<(), remu_state::StateError> {
    match op {
        OpIviInst::Vmerge_vim => {
            let simm5 = ((decoded.imm << 27) as i32) >> 27;
            let vm = decoded.rs1 != 0;
            vector_element_loop(
                ctx,
                decoded.rd as usize,
                if vm { None } else { Some(decoded.rs2 as usize) },
                mode_from_vm(vm),
                |_, sew, src, mask, _dst| {
                    if mask {
                        merge_scalar_vi(simm5, sew)
                    } else {
                        src.unwrap_or(0)
                    }
                },
            )
        }
        OpIviInst::Vmseq_vi => {
            let simm5 = ((decoded.imm << 27) as i32) >> 27;
            vector_mask_cmp::<P, C, _>(
                ctx,
                decoded.rd as usize,
                decoded.rs2 as usize,
                simm5 as i64,
                (decoded.imm >> 8) != 0,
                |a, b| a == b,
            )
        }
        OpIviInst::Vmsne_vi => {
            let simm5 = ((decoded.imm << 27) as i32) >> 27;
            vector_mask_cmp::<P, C, _>(
                ctx,
                decoded.rd as usize,
                decoded.rs2 as usize,
                simm5 as i64,
                (decoded.imm >> 8) != 0,
                |a, b| a != b,
            )
        }
        OpIviInst::Vmsle_vi => {
            let simm5 = ((decoded.imm << 27) as i32) >> 27;
            vector_mask_cmp::<P, C, _>(
                ctx,
                decoded.rd as usize,
                decoded.rs2 as usize,
                simm5 as i64,
                (decoded.imm >> 8) != 0,
                |a, b| a <= b,
            )
        }
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
            vector_element_loop(
                ctx,
                decoded.rd as usize,
                Some(decoded.rs2 as usize),
                mode_from_vm(decoded.rs1 != 0),
                |_, sew, src, mask, dst| {
                    if mask {
                        binop_sub_vi(simm5, src.unwrap_or(0), sew)
                    } else {
                        dst
                    }
                },
            )
        }
        OpIviInst::Vadd_vi => {
            let simm5 = ((decoded.imm << 27) as i32) >> 27;
            vector_element_loop(
                ctx,
                decoded.rd as usize,
                Some(decoded.rs2 as usize),
                mode_from_vm(decoded.rs1 != 0),
                |_, sew, src, mask, dst| {
                    if mask {
                        binop_add_vi(simm5, src.unwrap_or(0), sew)
                    } else {
                        dst
                    }
                },
            )
        }
        OpIviInst::Vslidedown_vi => vector_slide::<P, C>(
            ctx,
            decoded.rd as usize,
            decoded.rs2 as usize,
            (decoded.imm & 0x1F) as usize,
            mode_from_vm((decoded.imm >> 8) != 0),
        ),
        OpIviInst::Vslideup_vi => vector_slide_up::<P, C>(
            ctx,
            decoded.rd as usize,
            decoded.rs2 as usize,
            (decoded.imm & 0x1F) as usize,
            mode_from_vm((decoded.imm >> 8) != 0),
        ),
        OpIviInst::Vsll_vi => {
            let uimm5 = decoded.imm & 0x1F;
            vector_element_loop(
                ctx,
                decoded.rd as usize,
                Some(decoded.rs2 as usize),
                mode_from_vm((decoded.imm >> 8) != 0),
                |_, sew, src, mask, dst| {
                    if mask {
                        binop_shl_vi(uimm5, src.unwrap_or(0), sew)
                    } else {
                        dst
                    }
                },
            )
        }
        OpIviInst::Vsrl_vi => {
            let uimm5 = decoded.imm & 0x1F;
            vector_element_loop(
                ctx,
                decoded.rd as usize,
                Some(decoded.rs2 as usize),
                mode_from_vm((decoded.imm >> 8) != 0),
                |_, sew, src, mask, dst| {
                    if mask {
                        binop_shr_vi(uimm5, src.unwrap_or(0), sew)
                    } else {
                        dst
                    }
                },
            )
        }
        OpIviInst::Vand_vi => {
            let simm5 = decoded.imm as i32;
            vector_element_loop(
                ctx,
                decoded.rd as usize,
                Some(decoded.rs2 as usize),
                mode_from_vm(decoded.rs1 != 0),
                |_, sew, src, mask, dst| {
                    if mask {
                        binop_and_vi(simm5, src.unwrap_or(0), sew)
                    } else {
                        dst
                    }
                },
            )
        }
    }
}
