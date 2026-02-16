#![allow(non_snake_case)]

use remu_state::{StateError, StatePolicy};
use remu_types::isa::RvIsa;
use remu_types::isa::extension_v::VExtensionConfig;

remu_macro::mod_pub!(opcode);
remu_macro::mod_flat!(bytes);

use crate::riscv::inst::opcode::{
    AUIPC, BRANCH, JAL, JALR, LOAD, LUI, MISC_MEM, OP, OP_IMM, OP_V, STORE, STORE_FP, SYSTEM, UNKNOWN,
};

/// Instruction kind: one variant per opcode, with opcode-specific sub-enum where needed.
#[derive(Clone, Copy, Debug, Default)]
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
    StoreFp(STORE_FP::StoreFpInst),
    MiscMem(MISC_MEM::MiscMemInst),
    System(SYSTEM::SystemInst),
    V(OP_V::VInst),
    #[default]
    Unknown,
}

#[derive(Clone, Copy, Default)]
pub struct DecodedInst {
    pub(crate) rs1: u8,
    pub(crate) rs2: u8,
    pub(crate) rd: u8,
    pub imm: u32,
    pub(crate) inst: Inst,
}

#[inline(always)]
pub fn decode<P: StatePolicy>(inst: u32) -> DecodedInst {
    let op = opcode(inst);
    match op {
        LUI::OPCODE => LUI::decode::<P>(inst),
        AUIPC::OPCODE => AUIPC::decode::<P>(inst),
        JAL::OPCODE => JAL::decode::<P>(inst),
        JALR::OPCODE => JALR::decode::<P>(inst),
        BRANCH::OPCODE => BRANCH::decode::<P>(inst),
        LOAD::OPCODE => LOAD::decode::<P>(inst),
        STORE::OPCODE => STORE::decode::<P>(inst),
        STORE_FP::OPCODE => STORE_FP::decode::<P>(inst),
        OP_IMM::OPCODE => OP_IMM::decode::<P>(inst),
        OP::OPCODE => OP::decode::<P>(inst),
        MISC_MEM::OPCODE => MISC_MEM::decode::<P>(inst),
        SYSTEM::OPCODE => SYSTEM::decode::<P>(inst),
        OP_V::OPCODE => {
            if <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB > 0 {
                OP_V::decode::<P>(inst)
            } else {
                UNKNOWN::decode::<P>(inst)
            }
        }
        _ => UNKNOWN::decode::<P>(inst),
    }
}

#[inline(always)]
pub(crate) fn execute<P: StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
) -> Result<(), StateError> {
    match decoded.inst {
        Inst::Lui => LUI::execute(ctx, decoded),
        Inst::Auipc => AUIPC::execute(ctx, decoded),
        Inst::Jal => JAL::execute(ctx, decoded),
        Inst::Jalr => JALR::execute(ctx, decoded),
        Inst::Branch(..) => BRANCH::execute(ctx, decoded),
        Inst::OpImm(..) => OP_IMM::execute(ctx, decoded),
        Inst::Op(..) => OP::execute(ctx, decoded),
        Inst::Load(..) => LOAD::execute(ctx, decoded),
        Inst::Store(..) => STORE::execute(ctx, decoded),
        Inst::StoreFp(..) => STORE_FP::execute(ctx, decoded),
        Inst::MiscMem(..) => MISC_MEM::execute(ctx, decoded),
        Inst::System(..) => SYSTEM::execute(ctx, decoded),
        Inst::V(..) => {
            if <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB > 0 {
                OP_V::execute(ctx, decoded)
            } else {
                unsafe { core::hint::unreachable_unchecked() }
            }
        }
        Inst::Unknown => UNKNOWN::execute(ctx, decoded),
    }
}

pub const RV32_INSTRUCTION_MIX: &[(u32, u32)] = &[
    (AUIPC::OPCODE, AUIPC::INSTRUCTION_MIX),
    (BRANCH::OPCODE, BRANCH::INSTRUCTION_MIX),
    (JAL::OPCODE, JAL::INSTRUCTION_MIX),
    (JALR::OPCODE, JALR::INSTRUCTION_MIX),
    (LOAD::OPCODE, LOAD::INSTRUCTION_MIX),
    (LUI::OPCODE, LUI::INSTRUCTION_MIX),
    (MISC_MEM::OPCODE, MISC_MEM::INSTRUCTION_MIX),
    (OP::OPCODE, OP::INSTRUCTION_MIX),
    (OP_IMM::OPCODE, OP_IMM::INSTRUCTION_MIX),
    (STORE::OPCODE, STORE::INSTRUCTION_MIX),
    (STORE_FP::OPCODE, STORE_FP::INSTRUCTION_MIX),
    (SYSTEM::OPCODE, SYSTEM::INSTRUCTION_MIX),
    (OP_V::OPCODE, OP_V::INSTRUCTION_MIX),
    (UNKNOWN::OPCODE, UNKNOWN::INSTRUCTION_MIX),
];
