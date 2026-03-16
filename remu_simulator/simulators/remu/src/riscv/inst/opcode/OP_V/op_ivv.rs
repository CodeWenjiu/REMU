//! funct3 = 0b000: OP-IVV (vector-vector)

use crate::riscv::inst::{DecodedInst, opcode::OP_V::OpIvvInst};

use super::{
    loop_ops::mode_from_vm,
    utils::vector_element_loop_vv,
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
    }
}
