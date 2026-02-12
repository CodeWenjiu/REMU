use remu_types::isa::reg::RegAccess;

use crate::riscv::inst::{imm_u, rd, DecodedInst, Inst};

pub(crate) const OPCODE: u32 = 0b001_0111;
pub(crate) const INSTRUCTION_MIX: u32 = 20;

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    DecodedInst {
        rs1: 0,
        rs2: 0,
        rd: rd(inst),
        imm: imm_u(inst),
        inst: Inst::Auipc,
    }
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy>(
    state: &mut remu_state::State<P>,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    let value: u32 = state.reg.pc.wrapping_add(decoded.imm);
    state.reg.gpr.raw_write(decoded.rd.into(), value);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}
