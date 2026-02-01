use std::marker::PhantomData;

use remu_state::{State, StateError, bus::BusObserver};
use remu_types::isa::{RvIsa, reg::RegAccess};

use crate::riscv::inst::{DecodedInst, SimulatorError, funct3, imm_s, rs1, rs2};

pub(crate) const OPCODE: u32 = 0b010_0011;

pub(crate) const INSTRUCTION_MIX: u32 = 110;

mod func3 {
    pub const SB: u32 = 0b000; // Store Byte
    pub const SH: u32 = 0b001; // Store Halfword
    pub const SW: u32 = 0b010; // Store Word
}

fn sb<I: RvIsa, O: BusObserver>(
    state: &mut State<I>,
    inst: &DecodedInst<I, O>,
    obs: &mut O,
) -> Result<(), SimulatorError> {
    let rs1 = state.reg.gpr.raw_read(inst.rs1.into());
    let addr = rs1.wrapping_add(inst.imm);
    state
        .bus
        .write_8(
            addr as usize,
            state.reg.gpr.raw_read(inst.rs2.into()) as u8,
            obs,
        )
        .map_err(StateError::from)?;
    state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

fn sh<I: RvIsa, O: BusObserver>(
    state: &mut State<I>,
    inst: &DecodedInst<I, O>,
    obs: &mut O,
) -> Result<(), SimulatorError> {
    let rs1 = state.reg.gpr.raw_read(inst.rs1.into());
    let addr = rs1.wrapping_add(inst.imm);
    state
        .bus
        .write_16(
            addr as usize,
            state.reg.gpr.raw_read(inst.rs2.into()) as u16,
            obs,
        )
        .map_err(StateError::from)?;
    state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

fn sw<I: RvIsa, O: BusObserver>(
    state: &mut State<I>,
    inst: &DecodedInst<I, O>,
    obs: &mut O,
) -> Result<(), SimulatorError> {
    let rs1 = state.reg.gpr.raw_read(inst.rs1.into());
    let addr = rs1.wrapping_add(inst.imm);
    state
        .bus
        .write_32(
            addr as usize,
            state.reg.gpr.raw_read(inst.rs2.into()),
            obs,
        )
        .map_err(StateError::from)?;
    state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

#[inline(always)]
pub(crate) fn decode<I: RvIsa, O: BusObserver>(inst: u32) -> DecodedInst<I, O> {
    let f3 = funct3(inst);

    let rs1 = rs1(inst);
    let rs2 = rs2(inst);
    let imm = imm_s(inst);

    match f3 {
        func3::SB => DecodedInst::<I, O> {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: sb::<I, O>,
            _marker: PhantomData,
        },
        func3::SH => DecodedInst::<I, O> {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: sh::<I, O>,
            _marker: PhantomData,
        },
        func3::SW => DecodedInst::<I, O> {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: sw::<I, O>,
            _marker: PhantomData,
        },
        _ => DecodedInst::<I, O>::default(),
    }
}
