use remu_state::State;

use crate::inst::{DecodedInst, SimulatorError, imm_i, rd, rs1};

pub(crate) const OPCODE: u32 = 0b110_0111;

pub(crate) const INSTRUCTION_MIX: u32 = 30;

fn jalr(state: &mut State, inst: &DecodedInst) -> Result<(), SimulatorError> {
    let value: u32 = state.reg.pc.wrapping_add(4);
    state.reg.write_gpr(inst.rd.into(), value);
    state.reg.pc = state.reg.read_gpr(inst.rs1.into()).wrapping_add(inst.imm);
    Ok(())
}

#[inline(always)]
pub(crate) fn decode(inst: u32) -> DecodedInst {
    DecodedInst {
        rs1: rs1(inst),
        rs2: 0,
        rd: rd(inst),
        imm: imm_i(inst),
        handler: jalr,
    }
}
