use crate::riscv::{
    DecodedInst, Inst,
    opcode::OP_V::{OpMvvInst, VInst},
    opcode::UNKNOWN,
};

pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    _pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let v = match decoded.inst {
        Inst::V(v) => v,
        _ => return UNKNOWN::execute::<P, C>(ctx, decoded, _pc),
    };

    let state = ctx.state_mut();
    if state.reg.csr.mstatus_vs_off() {
        return Ok(UNKNOWN::trap_illegal_instruction(state, _pc));
    }

    // Only `vmv.x.s` / `vfirst.m` read vector state and write GPR; they do not update VS to Dirty.
    let dirties_vs = !matches!(
        v,
        VInst::OpMvv(OpMvvInst::Vmv_x_s) | VInst::OpMvv(OpMvvInst::Vfirst_m)
    );

    let r: Result<(), remu_state::StateError> = match v {
        VInst::OpCfg(op) => super::op_cfg::execute(ctx, decoded, op),
        VInst::OpIvv(op) => super::op_ivv::execute(ctx, decoded, op),
        VInst::OpMvv(op) => super::op_mvv::execute(ctx, decoded, op),
        VInst::OpIvi(op) => super::op_ivi::execute(ctx, decoded, op),
        VInst::OpIvx(op) => super::op_ivx::execute(ctx, decoded, op),
        VInst::OpMvx(op) => super::op_mvx::execute(ctx, decoded, op),
    };

    if r.is_ok() && dirties_vs {
        let state = ctx.state_mut();
        state.reg.csr.set_mstatus_vs_dirty();
        Ok(*state.reg.pc)
    } else {
        r.map(|()| *ctx.state_mut().reg.pc)
    }
}
