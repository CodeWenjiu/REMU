use std::marker::PhantomData;

use remu_state::{State, bus::BusObserver};
use remu_types::isa::{RvIsa, reg::RegAccess};

use crate::riscv::inst::{DecodedInst, SimulatorError, imm_u, rd};

pub(crate) const OPCODE: u32 = 0b011_0111;

pub(crate) const INSTRUCTION_MIX: u32 = 50;

fn lui<I: RvIsa, O: BusObserver>(
    state: &mut State<I>,
    inst: &DecodedInst<I, O>,
    _obs: &mut O,
) -> Result<(), SimulatorError> {
    let value: u32 = inst.imm;
    state.reg.gpr.raw_write(inst.rd.into(), value);
    state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

#[inline(always)]
pub(crate) fn decode<I: RvIsa, O: BusObserver>(inst: u32) -> DecodedInst<I, O> {
    DecodedInst::<I, O> {
        rs1: 0,
        rs2: 0,
        rd: rd(inst),
        imm: imm_u(inst),
        handler: lui::<I, O>,
        _marker: PhantomData,
    }
}
