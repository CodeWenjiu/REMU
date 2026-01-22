#![allow(non_snake_case)]

use remu_state::{State, bus::BusFault};
use thiserror::Error;
remu_macro::mod_flat!(
    bytes, LUI, AUIPC, JAL, JALR, BRANCH, OP_IMM, OP, LOAD, STORE
);

#[derive(Debug, Error)]
pub enum SimulatorError {
    #[error("Memory access error {0}")]
    MemoryAccessError(#[from] BusFault),
}

#[derive(Clone, Copy)]
pub struct DecodedInst {
    pub rs1: u8,
    pub rs2: u8,
    pub rd: u8,
    pub imm: u32,

    pub handler: fn(&mut State, &DecodedInst) -> Result<(), SimulatorError>,
}

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

// WIP
fn trap_unknown_instruction(state: &mut State, inst: &DecodedInst) -> Result<(), SimulatorError> {
    let _ = state;
    let _ = inst;
    Ok(())
}

impl Default for DecodedInst {
    fn default() -> Self {
        Self {
            rs1: 0,
            rs2: 0,
            rd: 0,
            imm: 0,
            handler: trap_unknown_instruction,
        }
    }
}
