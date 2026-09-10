use remu_isa::isa::reg::RegAccess;

use crate::riscv::{DecodedInst, Inst, imm_i, rd, rs1};

#[allow(dead_code)]
pub(crate) const OPCODE: u32 = 0b110_0111;
#[allow(dead_code)]
pub(crate) const INSTRUCTION_MIX: u32 = 30;

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    DecodedInst {
        rs1: rs1(inst),
        rs2: 0,
        rd: rd(inst),
        imm: imm_i(inst),
        inst: Inst::Jalr,
    }
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    state
        .reg
        .gpr
        .raw_write(decoded.rd.into(), pc.wrapping_add(4));
    Ok(rs1_val.wrapping_add(decoded.imm) & !1)
}
