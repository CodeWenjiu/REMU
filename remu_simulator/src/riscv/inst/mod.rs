#![allow(non_snake_case)]

use remu_state::{State, bus::BusFault};
use thiserror::Error;
remu_macro::mod_pub!(opcode);
remu_macro::mod_flat!(bytes);

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
