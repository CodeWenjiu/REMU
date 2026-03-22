//! funct3 = 0b100: OP-IVX

use remu_types::isa::reg::RegAccess;

use crate::riscv::inst::{DecodedInst, opcode::OP_V::OpIvxInst};

use super::{
    loop_ops::{
        binop_add_vx, binop_and_vx, binop_shl_vx, binop_shr_vx, merge_scalar_vx, mode_from_vm,
        scalar_sext,
    },
    utils::{vector_element_loop, vector_mask_cmp},
    VContext,
};

pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    op: OpIvxInst,
) -> Result<(), remu_state::StateError> {
    match op {
        OpIvxInst::Vmerge_vxm => {
            let scalar = ctx.state_mut().reg.gpr.raw_read(decoded.rs1.into());
            vector_element_loop(
                ctx,
                decoded.rd as usize,
                Some(decoded.rs2 as usize),
                mode_from_vm(decoded.imm != 0),
                |_, sew, src, mask, _dst| {
                    if mask {
                        merge_scalar_vx(scalar, sew)
                    } else {
                        src.unwrap_or(0)
                    }
                },
            )
        }
        OpIvxInst::Vadd_vx => {
            let scalar = ctx.state_mut().reg.gpr.raw_read(decoded.rs1.into()) as u64;
            vector_element_loop(
                ctx,
                decoded.rd as usize,
                Some(decoded.rs2 as usize),
                mode_from_vm(decoded.imm != 0),
                |_, sew, src, mask, dst| {
                    if mask {
                        binop_add_vx(scalar, src.unwrap_or(0), sew)
                    } else {
                        dst
                    }
                },
            )
        }
        OpIvxInst::Vand_vx => {
            let scalar = ctx.state_mut().reg.gpr.raw_read(decoded.rs1.into()) as u64;
            vector_element_loop(
                ctx,
                decoded.rd as usize,
                Some(decoded.rs2 as usize),
                mode_from_vm(decoded.imm != 0),
                |_, sew, src, mask, dst| {
                    if mask {
                        binop_and_vx(scalar, src.unwrap_or(0), sew)
                    } else {
                        dst
                    }
                },
            )
        }
        OpIvxInst::Vmslt_vx => {
            let vctx = VContext::from_state::<P, C>(ctx);
            let scalar = ctx.state_mut().reg.gpr.raw_read(decoded.rs1.into());
            vector_mask_cmp::<P, C, _>(
                ctx,
                decoded.rd as usize,
                decoded.rs2 as usize,
                scalar_sext(scalar, vctx.sew_bytes),
                decoded.imm != 0,
                |a, b| a < b,
            )
        }
        OpIvxInst::Vmseq_vx => {
            let vctx = VContext::from_state::<P, C>(ctx);
            let scalar = ctx.state_mut().reg.gpr.raw_read(decoded.rs1.into());
            vector_mask_cmp::<P, C, _>(
                ctx,
                decoded.rd as usize,
                decoded.rs2 as usize,
                scalar_sext(scalar, vctx.sew_bytes),
                decoded.imm != 0,
                |a, b| a == b,
            )
        }
        OpIvxInst::Vsll_vx => {
            let rs1 = ctx.state_mut().reg.gpr.raw_read(decoded.rs1.into());
            vector_element_loop(
                ctx,
                decoded.rd as usize,
                Some(decoded.rs2 as usize),
                mode_from_vm(decoded.imm != 0),
                |_, sew, src, mask, dst| {
                    if mask {
                        binop_shl_vx(rs1, src.unwrap_or(0), sew)
                    } else {
                        dst
                    }
                },
            )
        }
        OpIvxInst::Vsrl_vx => {
            let rs1 = ctx.state_mut().reg.gpr.raw_read(decoded.rs1.into());
            vector_element_loop(
                ctx,
                decoded.rd as usize,
                Some(decoded.rs2 as usize),
                mode_from_vm(decoded.imm != 0),
                |_, sew, src, mask, dst| {
                    if mask {
                        binop_shr_vx(rs1, src.unwrap_or(0), sew)
                    } else {
                        dst
                    }
                },
            )
        }
    }
}
