use remu_state::State;
use remu_types::RvIsa;

use crate::riscv::inst::{DecodedInst, SimulatorError, imm_i, rd, rs1};

pub(crate) const OPCODE: u32 = 0b110_0111;

pub(crate) const INSTRUCTION_MIX: u32 = 30;

fn jalr<I: RvIsa>(state: &mut State<I>, inst: &DecodedInst<I>) -> Result<(), SimulatorError> {
    let value: u32 = state.reg.read_pc().wrapping_add(4);
    state.reg.write_gpr(inst.rd.into(), value);
    state
        .reg
        .write_pc(state.reg.read_gpr(inst.rs1.into()).wrapping_add(inst.imm));
    Ok(())
}

#[inline(always)]
pub(crate) fn decode<I: RvIsa>(inst: u32) -> DecodedInst<I> {
    DecodedInst {
        rs1: rs1(inst),
        rs2: 0,
        rd: rd(inst),
        imm: imm_i(inst),
        handler: jalr,
    }
}
