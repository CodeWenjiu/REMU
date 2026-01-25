use remu_state::State;
use remu_types::Rv32Isa;

use crate::riscv::inst::{DecodedInst, SimulatorError, imm_j, rd};

pub(crate) const OPCODE: u32 = 0b110_1111;

pub(crate) const INSTRUCTION_MIX: u32 = 30;

fn jal<I: Rv32Isa>(state: &mut State<I>, inst: &DecodedInst<I>) -> Result<(), SimulatorError> {
    let value: u32 = state.reg.pc.wrapping_add(4);
    state.reg.write_gpr(inst.rd.into(), value);
    state.reg.pc = state.reg.pc.wrapping_add(inst.imm);
    Ok(())
}

#[inline(always)]
pub(crate) fn decode<I: Rv32Isa>(inst: u32) -> DecodedInst<I> {
    DecodedInst {
        rs1: 0,
        rs2: 0,
        rd: rd(inst),
        imm: imm_j(inst),
        handler: jal,
    }
}
