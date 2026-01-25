#![allow(non_snake_case)]

use remu_types::RvIsa;

use crate::riscv::inst::{DecodedInst, opcode};
remu_macro::mod_flat!(
    LUI, AUIPC, JAL, JALR, BRANCH, OP_IMM, OP, LOAD, STORE, UNKNOWN
);

#[inline(always)]
pub fn decode<I: RvIsa>(inst: u32) -> DecodedInst<I> {
    let opcode = opcode(inst);
    match opcode {
        LUI::OPCODE => LUI::decode(inst),
        AUIPC::OPCODE => AUIPC::decode(inst),
        JAL::OPCODE => JAL::decode(inst),
        JALR::OPCODE => JALR::decode(inst),
        BRANCH::OPCODE => BRANCH::decode(inst),
        LOAD::OPCODE => LOAD::decode(inst),
        STORE::OPCODE => STORE::decode(inst),
        OP_IMM::OPCODE => OP_IMM::decode(inst),
        OP::OPCODE => OP::decode(inst),
        _ => DecodedInst::default(),
    }
}

pub const RV32_INSTRUCTION_MIX: &[(u32, u32)] = &[
    // (Base Opcode, Permille Weight)
    (AUIPC::OPCODE, AUIPC::INSTRUCTION_MIX),
    (BRANCH::OPCODE, BRANCH::INSTRUCTION_MIX),
    (JAL::OPCODE, JAL::INSTRUCTION_MIX),
    (JALR::OPCODE, JALR::INSTRUCTION_MIX),
    (LOAD::OPCODE, LOAD::INSTRUCTION_MIX),
    (LUI::OPCODE, LUI::INSTRUCTION_MIX),
    (OP::OPCODE, OP::INSTRUCTION_MIX),
    (OP_IMM::OPCODE, OP_IMM::INSTRUCTION_MIX),
    (STORE::OPCODE, STORE::INSTRUCTION_MIX),
    (UNKNOWN::OPCODE, UNKNOWN::INSTRUCTION_MIX),
];
