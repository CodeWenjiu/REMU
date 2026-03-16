//! funct3 = 0b110: OP-MVX (vmv.s.x, vwmul.vx)

use remu_types::isa::reg::RegAccess;

use crate::riscv::inst::{DecodedInst, opcode::OP_V::OpMvxInst};

use super::{
    loop_ops::scalar_sext,
    utils::{vector_insert_scalar, vector_wide_mul_vx},
    VContext,
};

pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    op: OpMvxInst,
) -> Result<(), remu_state::StateError> {
    match op {
        OpMvxInst::Vwmul_vx => {
            let vctx = VContext::from_state::<P, C>(ctx);
            let scalar = ctx.state_mut().reg.gpr.raw_read(decoded.rs1.into());
            vector_wide_mul_vx::<P, C>(
                ctx,
                decoded.rd as usize,
                decoded.rs2 as usize,
                scalar_sext(scalar, vctx.sew_bytes),
                decoded.imm != 0,
            )
        }
        OpMvxInst::Vmv_s_x => {
            let scalar = ctx.state_mut().reg.gpr.raw_read(decoded.rs1.into());
            vector_insert_scalar::<P, C>(ctx, decoded.rd as usize, scalar)
        }
    }
}
