use std::marker::PhantomData;

use remu_state::{State, bus::BusObserver};
use remu_types::isa::{RvIsa, reg::RegAccess};

use crate::riscv::inst::{DecodedInst, SimulatorError, imm_i, rd, rs1};

pub(crate) const OPCODE: u32 = 0b110_0111;

pub(crate) const INSTRUCTION_MIX: u32 = 30;

fn jalr<I: RvIsa, O: BusObserver>(
    state: &mut State<I>,
    inst: &DecodedInst<I, O>,
    _obs: &mut O,
) -> Result<(), SimulatorError> {
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
pub(crate) fn decode<I: RvIsa, O: BusObserver>(inst: u32) -> DecodedInst<I, O> {
    DecodedInst::<I, O> {
        rs1: rs1(inst),
        rs2: 0,
        rd: rd(inst),
        imm: imm_i(inst),
        handler: jalr::<I, O>,
        _marker: PhantomData,
    }
}
