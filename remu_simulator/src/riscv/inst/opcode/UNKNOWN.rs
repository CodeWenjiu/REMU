use remu_state::State;

use crate::riscv::inst::{DecodedInst, SimulatorError};

pub(crate) const OPCODE: u32 = 0b111_1111;

pub(crate) const INSTRUCTION_MIX: u32 = 2;

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
