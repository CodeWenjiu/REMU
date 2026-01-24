#![allow(non_snake_case)]

use crate::inst::{DecodedInst, opcode};
remu_macro::mod_flat!(
    LUI, AUIPC, JAL, JALR, BRANCH, OP_IMM, OP, LOAD, STORE, UNKNOWN
);

#[inline(always)]
pub fn decode(inst: u32) -> DecodedInst {
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
