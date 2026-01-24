use remu_state::State;

use crate::inst::{DecodedInst, SimulatorError, imm_u, rd};

pub(crate) const OPCODE: u32 = 0b011_0111;

#[allow(unused)]
pub(crate) const INSTRUCTION_MIX: u32 = 50;

fn lui(state: &mut State, inst: &DecodedInst) -> Result<(), SimulatorError> {
    let value: u32 = inst.imm;
    state.reg.write_gpr(inst.rd.into(), value);
    state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

#[inline(always)]
pub(crate) fn decode(inst: u32) -> DecodedInst {
    DecodedInst {
        rs1: 0,
        rs2: 0,
        rd: rd(inst),
        imm: imm_u(inst),
        handler: lui,
    }
}
