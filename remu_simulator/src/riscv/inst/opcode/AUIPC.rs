use remu_state::State;
use remu_types::RvIsa;

use crate::riscv::inst::{DecodedInst, SimulatorError, imm_u, rd};

pub(crate) const OPCODE: u32 = 0b001_0111;

pub(crate) const INSTRUCTION_MIX: u32 = 20;

fn auipc<I: RvIsa>(state: &mut State<I>, inst: &DecodedInst<I>) -> Result<(), SimulatorError> {
    let value: u32 = state.reg.pc.wrapping_add(inst.imm);
    state.reg.write_gpr(inst.rd.into(), value);
    state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

#[inline(always)]
pub(crate) fn decode<I: RvIsa>(inst: u32) -> DecodedInst<I> {
    DecodedInst {
        rs1: 0,
        rs2: 0,
        rd: rd(inst),
        imm: imm_u(inst),
        handler: auipc,
    }
}
