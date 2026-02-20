//! funct3 = 0b000: OP-IVV (vector-vector)

use crate::riscv::inst::{DecodedInst, opcode::OP_V::OpIvvInst};

use super::utils::{vector_element_loop_vv, VectorElementLoopMode};

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    op: OpIvvInst,
) -> Result<(), remu_state::StateError> {
    match op {
        OpIvvInst::Vor_vv => {
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
                |_, _sew_bytes, src1, src2, mask, dst| {
                    if mask {
                        src2 | src1
                    } else {
                        dst
                    }
                },
            )
        }
    }
}
