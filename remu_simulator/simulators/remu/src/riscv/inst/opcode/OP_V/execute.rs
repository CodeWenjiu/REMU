use crate::riscv::inst::{DecodedInst, Inst, opcode::OP_V::VInst};

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    let v = match decoded.inst {
        Inst::V(v) => v,
        _ => return crate::riscv::inst::opcode::UNKNOWN::execute::<P, C>(ctx, decoded),
    };

    match v {
        VInst::OpCfg(op) => super::op_cfg::execute(ctx, decoded, op),
        VInst::OpIvv(op) => super::op_ivv::execute(ctx, decoded, op),
        VInst::OpMvv(op) => super::op_mvv::execute(ctx, decoded, op),
        VInst::OpIvi(op) => super::op_ivi::execute(ctx, decoded, op),
        VInst::OpIvx(op) => super::op_ivx::execute(ctx, decoded, op),
        VInst::OpMvx(op) => super::op_mvx::execute(ctx, decoded, op),
    }
}
