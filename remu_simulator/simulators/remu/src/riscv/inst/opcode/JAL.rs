use remu_types::isa::reg::RegAccess;

use crate::riscv::inst::{imm_j, rd, DecodedInst, Inst};

pub(crate) const OPCODE: u32 = 0b110_1111;
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
pub(crate) fn execute<P: remu_state::StatePolicy>(
    state: &mut remu_state::State<P>,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    let value: u32 = state.reg.pc.wrapping_add(4);
    state.reg.gpr.raw_write(decoded.rd.into(), value);
    *state.reg.pc = state.reg.pc.wrapping_add(decoded.imm);
    Ok(())
}
