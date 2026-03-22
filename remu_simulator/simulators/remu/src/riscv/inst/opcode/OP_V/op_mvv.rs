//! funct3 = 0b010: OP-MVV


use crate::riscv::inst::{DecodedInst, opcode::OP_V::OpMvvInst};

use super::{
    loop_ops::{binop_macc, mode_from_vm},
    utils::{
        vector_element_loop, vector_element_loop_vv, vector_extend_vf2, vector_extend_vf4,
        vector_extract_scalar,
        vector_first_mask, vector_mask_binary, vector_reduction, vector_wide_mul_vv,
    },
};

pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    op: OpMvvInst,
) -> Result<(), remu_state::StateError> {
    match op {
        OpMvvInst::Vredsum_vs => vector_reduction::<P, C, _>(
            ctx,
            decoded.rd as usize,
            decoded.rs1 as usize,
            decoded.rs2 as usize,
            decoded.imm != 0,
            |acc, v| acc.wrapping_add(v),
        ),
        OpMvvInst::Vredmax_vs => vector_reduction::<P, C, _>(
            ctx,
            decoded.rd as usize,
            decoded.rs1 as usize,
            decoded.rs2 as usize,
            decoded.imm != 0,
            |acc, v| if acc >= v { acc } else { v },
        ),
        OpMvvInst::Vid_v => vector_element_loop(
            ctx,
            decoded.rd as usize,
            None,
            mode_from_vm(true),
            |idx, _, _, _mask, _dst| idx as u64,
        ),
        OpMvvInst::Vmv_x_s => vector_extract_scalar::<P, C>(ctx, decoded.rd, decoded.rs2 as usize),
        OpMvvInst::Vfirst_m => vector_first_mask::<P, C>(ctx, decoded.rd, decoded.rs2 as usize),
        OpMvvInst::Vsext_vf4 => vector_extend_vf4::<P, C>(
            ctx,
            decoded.rd as usize,
            decoded.rs2 as usize,
            decoded.imm != 0,
            true,
        ),
        OpMvvInst::Vzext_vf4 => vector_extend_vf4::<P, C>(
            ctx,
            decoded.rd as usize,
            decoded.rs2 as usize,
            decoded.imm != 0,
            false,
        ),
        OpMvvInst::Vsext_vf2 => vector_extend_vf2::<P, C>(
            ctx,
            decoded.rd as usize,
            decoded.rs2 as usize,
            decoded.imm != 0,
            true,
        ),
        OpMvvInst::Vzext_vf2 => vector_extend_vf2::<P, C>(
            ctx,
            decoded.rd as usize,
            decoded.rs2 as usize,
            decoded.imm != 0,
            false,
        ),
        OpMvvInst::Vmor_mm => vector_mask_binary::<P, C, _>(
            ctx,
            decoded.rd as usize,
            decoded.rs1 as usize,
            decoded.rs2 as usize,
            |a, b| a | b,
        ),
        OpMvvInst::Vmacc_vv => {
            vector_element_loop_vv(
                ctx,
                decoded.rd as usize,
                decoded.rs1 as usize,
                decoded.rs2 as usize,
                mode_from_vm(decoded.imm != 0),
                |_, sew, src1, src2, mask, dst| {
                    if mask {
                        binop_macc(src1, src2, dst, sew)
                    } else {
                        dst
                    }
                },
            )
        }
        OpMvvInst::Vwmacc_vv => vector_wide_mul_vv::<P, C>(
            ctx,
            decoded.rd as usize,
            decoded.rs1 as usize,
            decoded.rs2 as usize,
            decoded.imm != 0,
            true,
            true,
        ),
        OpMvvInst::Vwmulu_vv => vector_wide_mul_vv::<P, C>(
            ctx,
            decoded.rd as usize,
            decoded.rs1 as usize,
            decoded.rs2 as usize,
            decoded.imm != 0,
            false,
            false,
        ),
    }
}
