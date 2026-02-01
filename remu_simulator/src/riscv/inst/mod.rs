#![allow(non_snake_case)]

use std::marker::PhantomData;

use remu_state::{State, bus::BusObserver};
use remu_types::isa::RvIsa;

use crate::riscv::SimulatorError;
remu_macro::mod_pub!(opcode);
remu_macro::mod_flat!(bytes);

#[derive(Clone, Copy)]
pub struct DecodedInst<I: RvIsa, O: BusObserver> {
    pub(crate) rs1: u8,
    pub(crate) rs2: u8,
    pub(crate) rd: u8,
    /// Public so the decode bench (separate crate) can use it; others use pub(crate).
    pub imm: u32,

    pub(crate) handler: fn(&mut State<I>, &DecodedInst<I, O>, &mut O) -> Result<(), SimulatorError>,
    pub(crate) _marker: PhantomData<fn(O) -> O>,
}
