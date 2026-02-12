use remu_types::isa::reg::RegAccess;

use crate::riscv::inst::{imm_u, rd, DecodedInst, Inst};

pub(crate) const OPCODE: u32 = 0b011_0111;
pub(crate) const INSTRUCTION_MIX: u32 = 50;

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    DecodedInst {
        rs1: 0,
        rs2: 0,
        rd: rd(inst),
        imm: imm_u(inst),
        inst: Inst::Lui,
    }
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    let state = ctx.state_mut();
    let value: u32 = decoded.imm;
    state.reg.gpr.raw_write(decoded.rd.into(), value);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}
