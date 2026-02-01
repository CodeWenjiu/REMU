use std::marker::PhantomData;

use remu_state::{State, StatePolicy};

use crate::riscv::inst::{DecodedInst, SimulatorError};

pub(crate) const OPCODE: u32 = 0b111_1111;

pub(crate) const INSTRUCTION_MIX: u32 = 2;

fn trap_unknown_instruction<P: StatePolicy>(
    state: &mut State<P>,
    inst: &DecodedInst<P>,
) -> Result<(), SimulatorError> {
    let _ = state;
    let _ = inst;
    Ok(())
}

impl<P: StatePolicy> Default for DecodedInst<P> {
    fn default() -> Self {
        Self {
            rs1: 0,
            rs2: 0,
            rd: 0,
            imm: 0,
            handler: trap_unknown_instruction::<P>,
            _marker: PhantomData,
        }
    }
}
