use remu_isa::isa::reg::RegAccess;

use crate::riscv::{DecodedInst, Inst, imm_j, rd};

#[allow(dead_code)]
pub(crate) const OPCODE: u32 = 0b110_1111;
#[allow(dead_code)]
pub(crate) const INSTRUCTION_MIX: u32 = 30;

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    DecodedInst {
        rs1: 0,
        rs2: 0,
        rd: rd(inst),
        imm: imm_j(inst),
        inst: Inst::Jal,
    }
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    let value: u32 = pc.wrapping_add(4);
    state.reg.gpr.raw_write(decoded.rd.into(), value);
    Ok(pc.wrapping_add(decoded.imm))
}
