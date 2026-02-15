//! RISC-V V extension (OP-V opcode 0x57). Decode only when VLENB > 0 (see inst/mod.rs).
//! Fill in decode and execute as needed.

use crate::riscv::inst::{rd, rs1, rs2, DecodedInst, Inst};

pub(crate) const OPCODE: u32 = 0b101_0111; // OP-V
pub(crate) const INSTRUCTION_MIX: u32 = 5;

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    DecodedInst {
        rd: rd(inst),
        rs1: rs1(inst),
        rs2: rs2(inst),
        imm: 0,
        inst: Inst::V(()),
    }
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    crate::riscv::inst::opcode::UNKNOWN::execute::<P, C>(ctx, decoded)
}
