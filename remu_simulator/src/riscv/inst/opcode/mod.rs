#![allow(non_snake_case)]

use remu_state::bus::BusObserver;
use remu_types::isa::RvIsa;

use crate::riscv::inst::{DecodedInst, opcode};
remu_macro::mod_flat!(
    LUI, AUIPC, JAL, JALR, BRANCH, OP_IMM, OP, LOAD, STORE, UNKNOWN
);

#[inline(always)]
pub fn decode<I: RvIsa, O: BusObserver>(inst: u32) -> DecodedInst<I, O> {
    let opcode = opcode(inst);
    match opcode {
        LUI::OPCODE => LUI::decode::<I, O>(inst),
        AUIPC::OPCODE => AUIPC::decode::<I, O>(inst),
        JAL::OPCODE => JAL::decode::<I, O>(inst),
        JALR::OPCODE => JALR::decode::<I, O>(inst),
        BRANCH::OPCODE => BRANCH::decode::<I, O>(inst),
        LOAD::OPCODE => LOAD::decode::<I, O>(inst),
        STORE::OPCODE => STORE::decode::<I, O>(inst),
        OP_IMM::OPCODE => OP_IMM::decode::<I, O>(inst),
        OP::OPCODE => OP::decode::<I, O>(inst),
        _ => DecodedInst::<I, O>::default(),
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
