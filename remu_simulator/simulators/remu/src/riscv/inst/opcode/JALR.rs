use remu_types::isa::reg::RegAccess;

use crate::riscv::inst::{imm_i, rd, rs1, DecodedInst, Inst};

pub(crate) const OPCODE: u32 = 0b110_0111;
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
pub(crate) fn execute<P: remu_state::StatePolicy>(
    state: &mut remu_state::State<P>,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    state
        .reg
        .gpr
        .raw_write(decoded.rd.into(), state.reg.pc.wrapping_add(4));
    *state.reg.pc = rs1_val.wrapping_add(decoded.imm) & !1;
    Ok(())
}
