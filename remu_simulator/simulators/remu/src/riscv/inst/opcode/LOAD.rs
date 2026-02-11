use std::marker::PhantomData;

use remu_state::StateError;
use remu_types::isa::reg::RegAccess;

use crate::riscv::inst::{funct3, imm_i, rd, rs1, DecodedInst, Inst};

pub(crate) const OPCODE: u32 = 0b000_0011;
pub(crate) const INSTRUCTION_MIX: u32 = 220;

mod func3 {
    pub(super) const LB: u32 = 0b000;
    pub(super) const LH: u32 = 0b001;
    pub(super) const LW: u32 = 0b010;
    pub(super) const LBU: u32 = 0b100;
    pub(super) const LHU: u32 = 0b101;
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum LoadInst {
    Lb,
    Lh,
    Lw,
    Lbu,
    Lhu,
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst<P> {
    let f3 = funct3(inst);
    let load = match f3 {
        func3::LB => LoadInst::Lb,
        func3::LH => LoadInst::Lh,
        func3::LW => LoadInst::Lw,
        func3::LBU => LoadInst::Lbu,
        func3::LHU => LoadInst::Lhu,
        _ => return DecodedInst::default(),
    };
    DecodedInst {
        rd: rd(inst),
        rs1: rs1(inst),
        rs2: 0,
        imm: imm_i(inst),
        inst: Inst::Load(load),
        _marker: PhantomData,
    }
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy>(
    state: &mut remu_state::State<P>,
    decoded: &DecodedInst<P>,
) -> Result<(), remu_state::StateError> {
    let Inst::Load(load) = decoded.inst else { unreachable!() };
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let addr = rs1_val.wrapping_add(decoded.imm);
    match load {
        LoadInst::Lb => {
            let v: u8 = state.bus.read_8(addr as usize).map_err(StateError::from)?;
            state.reg.gpr.raw_write(decoded.rd.into(), (v as i8) as u32);
        }
        LoadInst::Lh => {
            let v: u16 = state.bus.read_16(addr as usize).map_err(StateError::from)?;
            state.reg.gpr.raw_write(decoded.rd.into(), (v as i16) as u32);
        }
        LoadInst::Lw => {
            let v: u32 = state.bus.read_32(addr as usize).map_err(StateError::from)?;
            state.reg.gpr.raw_write(decoded.rd.into(), v);
        }
        LoadInst::Lbu => {
            let v: u8 = state.bus.read_8(addr as usize).map_err(StateError::from)?;
            state.reg.gpr.raw_write(decoded.rd.into(), v as u32);
        }
        LoadInst::Lhu => {
            let v: u16 = state.bus.read_16(addr as usize).map_err(StateError::from)?;
            state.reg.gpr.raw_write(decoded.rd.into(), v as u32);
        }
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}
