#![allow(non_snake_case)]

use std::marker::PhantomData;

use remu_state::{State, StatePolicy};

use crate::riscv::SimulatorInnerError;
remu_macro::mod_pub!(opcode);
remu_macro::mod_flat!(bytes);

#[derive(Clone, Copy)]
pub struct DecodedInst<P: StatePolicy> {
    pub(crate) rs1: u8,
    pub(crate) rs2: u8,
    pub(crate) rd: u8,
    pub imm: u32,

    pub(crate) handler: fn(&mut State<P>, &DecodedInst<P>) -> Result<(), SimulatorInnerError>,
    pub(crate) _marker: PhantomData<P>,
}
