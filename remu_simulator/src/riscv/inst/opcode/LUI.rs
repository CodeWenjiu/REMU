use std::marker::PhantomData;

use remu_state::{State, StatePolicy};
use remu_types::isa::reg::RegAccess;

use crate::riscv::inst::{DecodedInst, SimulatorError, imm_u, rd};

pub(crate) const OPCODE: u32 = 0b011_0111;

pub(crate) const INSTRUCTION_MIX: u32 = 50;

fn lui<P: StatePolicy>(state: &mut State<P>, inst: &DecodedInst<P>) -> Result<(), SimulatorError> {
    let value: u32 = inst.imm;
    state.reg.gpr.raw_write(inst.rd.into(), value);
    state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

#[inline(always)]
pub(crate) fn decode<P: StatePolicy>(inst: u32) -> DecodedInst<P> {
    DecodedInst::<P> {
        rs1: 0,
        rs2: 0,
        rd: rd(inst),
        imm: imm_u(inst),
        handler: lui::<P>,
        _marker: PhantomData,
    }
}
