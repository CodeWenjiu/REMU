#![allow(non_snake_case)]

use std::marker::PhantomData;

use remu_state::{State, StateError, StatePolicy};

remu_macro::mod_pub!(opcode);
remu_macro::mod_flat!(bytes);

use crate::riscv::inst::opcode::{
    AUIPC, BRANCH, JAL, JALR, LOAD, LUI, OP, OP_IMM, STORE, SYSTEM, UNKNOWN,
};

/// Instruction kind: one variant per opcode, with opcode-specific sub-enum where needed.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Inst {
    Lui,
    Auipc,
    Jal,
    Jalr,
    Branch(BRANCH::BranchInst),
    OpImm(OP_IMM::OpImmInst),
    Op(OP::OpInst),
    Load(LOAD::LoadInst),
    Store(STORE::StoreInst),
    System(SYSTEM::SystemInst),
    Unknown,
}

#[derive(Clone, Copy)]
pub struct DecodedInst<P: StatePolicy> {
    pub(crate) rs1: u8,
    pub(crate) rs2: u8,
    pub(crate) rd: u8,
    pub imm: u32,
    pub(crate) inst: Inst,
    pub(crate) _marker: PhantomData<P>,
}

impl<P: StatePolicy> Default for DecodedInst<P> {
    fn default() -> Self {
        Self {
            rs1: 0,
            rs2: 0,
            rd: 0,
            imm: 0,
            inst: Inst::Unknown,
            _marker: PhantomData,
        }
    }
}

#[inline(always)]
pub fn decode<P: StatePolicy>(inst: u32) -> DecodedInst<P> {
    let op = opcode(inst);
    match op {
        LUI::OPCODE => LUI::decode::<P>(inst),
        AUIPC::OPCODE => AUIPC::decode::<P>(inst),
        JAL::OPCODE => JAL::decode::<P>(inst),
        JALR::OPCODE => JALR::decode::<P>(inst),
        BRANCH::OPCODE => BRANCH::decode::<P>(inst),
        LOAD::OPCODE => LOAD::decode::<P>(inst),
        STORE::OPCODE => STORE::decode::<P>(inst),
        OP_IMM::OPCODE => OP_IMM::decode::<P>(inst),
        OP::OPCODE => OP::decode::<P>(inst),
        SYSTEM::OPCODE => SYSTEM::decode::<P>(inst),
        _ => UNKNOWN::decode::<P>(inst),
    }
}

#[inline(always)]
pub fn execute<P: StatePolicy>(
    state: &mut State<P>,
    decoded: &DecodedInst<P>,
) -> Result<(), StateError> {
    match decoded.inst {
        Inst::Lui => LUI::execute(state, decoded),
        Inst::Auipc => AUIPC::execute(state, decoded),
        Inst::Jal => JAL::execute(state, decoded),
        Inst::Jalr => JALR::execute(state, decoded),
        Inst::Branch(..) => BRANCH::execute(state, decoded),
        Inst::OpImm(..) => OP_IMM::execute(state, decoded),
        Inst::Op(..) => OP::execute(state, decoded),
        Inst::Load(..) => LOAD::execute(state, decoded),
        Inst::Store(..) => STORE::execute(state, decoded),
        Inst::System(..) => SYSTEM::execute(state, decoded),
        Inst::Unknown => UNKNOWN::execute(state, decoded),
    }
}

pub const RV32_INSTRUCTION_MIX: &[(u32, u32)] = &[
    (AUIPC::OPCODE, AUIPC::INSTRUCTION_MIX),
    (BRANCH::OPCODE, BRANCH::INSTRUCTION_MIX),
    (JAL::OPCODE, JAL::INSTRUCTION_MIX),
    (JALR::OPCODE, JALR::INSTRUCTION_MIX),
    (LOAD::OPCODE, LOAD::INSTRUCTION_MIX),
    (LUI::OPCODE, LUI::INSTRUCTION_MIX),
    (OP::OPCODE, OP::INSTRUCTION_MIX),
    (OP_IMM::OPCODE, OP_IMM::INSTRUCTION_MIX),
    (STORE::OPCODE, STORE::INSTRUCTION_MIX),
    (SYSTEM::OPCODE, SYSTEM::INSTRUCTION_MIX),
    (UNKNOWN::OPCODE, UNKNOWN::INSTRUCTION_MIX),
];
