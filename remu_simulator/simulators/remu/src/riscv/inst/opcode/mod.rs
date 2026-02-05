#![allow(non_snake_case)]

use remu_state::StatePolicy;

use crate::riscv::inst::{DecodedInst, opcode};

#[macro_export]
macro_rules! handler {
    ($name:ident, $state:ident, $inst:ident, $body:block) => {
        fn $name<P: remu_state::StatePolicy>(
            $state: &mut remu_state::State<P>,
            $inst: &$crate::riscv::inst::DecodedInst<P>,
        ) -> Result<(), remu_state::StateError> $body
    };
}

#[macro_export]
macro_rules! define_decode {
    ($inst:ident, $body:block) => {
        #[inline(always)]
        pub(crate) fn decode<P: remu_state::StatePolicy>(
            $inst: u32,
        ) -> $crate::riscv::inst::DecodedInst<P> $body
    };
}
remu_macro::mod_flat!(
    LUI, AUIPC, JAL, JALR, BRANCH, OP_IMM, OP, LOAD, STORE, UNKNOWN
);

#[inline(always)]
pub fn decode<P: StatePolicy>(inst: u32) -> DecodedInst<P> {
    let opcode = opcode(inst);
    match opcode {
        LUI::OPCODE => LUI::decode::<P>(inst),
        AUIPC::OPCODE => AUIPC::decode::<P>(inst),
        JAL::OPCODE => JAL::decode::<P>(inst),
        JALR::OPCODE => JALR::decode::<P>(inst),
        BRANCH::OPCODE => BRANCH::decode::<P>(inst),
        LOAD::OPCODE => LOAD::decode::<P>(inst),
        STORE::OPCODE => STORE::decode::<P>(inst),
        OP_IMM::OPCODE => OP_IMM::decode::<P>(inst),
        OP::OPCODE => OP::decode::<P>(inst),
        _ => DecodedInst::<P>::default(),
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
