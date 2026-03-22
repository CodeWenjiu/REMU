//! funct3 = 0b000: OP-IVV (vector-vector)

use crate::riscv::inst::{DecodedInst, opcode::OP_V::OpIvvInst};

use super::{
    loop_ops::{binop_max_vv, binop_sub_vv, mode_from_vm},
    utils::{vector_element_loop_vv, vector_mask_cmp_vv},
};

pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    op: OpIvvInst,
) -> Result<(), remu_state::StateError> {
    match op {
        OpIvvInst::Vor_vv => vector_element_loop_vv(
            ctx,
            decoded.rd as usize,
            decoded.rs1 as usize,
            decoded.rs2 as usize,
            mode_from_vm(decoded.imm != 0),
            |_, _, src1, src2, mask, dst| if mask { src1 | src2 } else { dst },
        ),
        OpIvvInst::Vsub_vv => vector_element_loop_vv(
            ctx,
            decoded.rd as usize,
            decoded.rs1 as usize,
            decoded.rs2 as usize,
            mode_from_vm(decoded.imm != 0),
            |_, sew, vs1, vs2, mask, dst| {
                if mask {
                    binop_sub_vv(vs1, vs2, sew)
                } else {
                    dst
                }
            },
        ),
        OpIvvInst::Vmax_vv => vector_element_loop_vv(
            ctx,
            decoded.rd as usize,
            decoded.rs1 as usize,
            decoded.rs2 as usize,
            mode_from_vm(decoded.imm != 0),
            |_, sew, vs1, vs2, mask, dst| {
                if mask {
                    binop_max_vv(vs1, vs2, sew)
                } else {
                    dst
                }
            },
        ),
        OpIvvInst::Vmsne_vv => vector_mask_cmp_vv::<P, C, _>(
            ctx,
            decoded.rd as usize,
            decoded.rs1 as usize,
            decoded.rs2 as usize,
            decoded.imm != 0,
            |vs1, vs2| vs1 != vs2,
        ),
    }
}
