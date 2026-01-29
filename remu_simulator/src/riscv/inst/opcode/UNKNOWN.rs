use std::marker::PhantomData;

use remu_state::{State, bus::BusObserver};
use remu_types::isa::RvIsa;

use crate::riscv::inst::{DecodedInst, SimulatorError};

pub(crate) const OPCODE: u32 = 0b111_1111;

pub(crate) const INSTRUCTION_MIX: u32 = 2;

// WIP
fn trap_unknown_instruction<I: RvIsa, O: BusObserver>(
    state: &mut State<I>,
    inst: &DecodedInst<I, O>,
) -> Result<(), SimulatorError> {
    let _ = state;
    let _ = inst;
    Ok(())
}

impl<I: RvIsa, O: BusObserver> Default for DecodedInst<I, O> {
    fn default() -> Self {
        Self {
            rs1: 0,
            rs2: 0,
            rd: 0,
            imm: 0,
            handler: trap_unknown_instruction::<I, O>,
            _marker: PhantomData,
        }
    }
}
