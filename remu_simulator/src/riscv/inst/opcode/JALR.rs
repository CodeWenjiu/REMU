use std::marker::PhantomData;

use remu_state::{State, StatePolicy};
use remu_types::isa::reg::RegAccess;

use crate::riscv::inst::{DecodedInst, SimulatorError, imm_i, rd, rs1};

pub(crate) const OPCODE: u32 = 0b110_0111;

pub(crate) const INSTRUCTION_MIX: u32 = 30;

fn jalr<P: StatePolicy>(state: &mut State<P>, inst: &DecodedInst<P>) -> Result<(), SimulatorError> {
    let value: u32 = state.reg.pc.wrapping_add(4);
    state.reg.gpr.raw_write(inst.rd.into(), value);
    state.reg.pc = state
        .reg
        .gpr
        .raw_read(inst.rs1.into())
        .wrapping_add(inst.imm);
    Ok(())
}

#[inline(always)]
pub(crate) fn decode<P: StatePolicy>(inst: u32) -> DecodedInst<P> {
    DecodedInst::<P> {
        rs1: rs1(inst),
        rs2: 0,
        rd: rd(inst),
        imm: imm_i(inst),
        handler: jalr::<P>,
        _marker: PhantomData,
    }
}
