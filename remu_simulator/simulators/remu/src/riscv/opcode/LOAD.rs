use remu_isa::isa::reg::RegAccess;
use remu_state::StateError;

use crate::riscv::{DecodedInst, Inst, funct3, imm_i, rd, rs1};

#[allow(dead_code)]
pub(crate) const OPCODE: u32 = 0b000_0011;
#[allow(dead_code)]
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
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
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
    }
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    let Inst::Load(load) = decoded.inst else {
        unreachable!()
    };
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let addr = rs1_val.wrapping_add(decoded.imm);
    match load {
        LoadInst::Lb => {
            let v: u8 = match state.bus.read_8_fast(addr as usize) {
                Some(v) => v,
                None => state
                    .bus
                    .read_8_slow_err(addr as usize)
                    .map_err(StateError::from)?,
            };
            state.reg.gpr.raw_write(decoded.rd.into(), (v as i8) as u32);
        }
        LoadInst::Lh => {
            let v: u16 = match state.bus.read_16_fast(addr as usize) {
                Some(v) => v,
                None => state
                    .bus
                    .read_16_slow_err(addr as usize)
                    .map_err(StateError::from)?,
            };
            state
                .reg
                .gpr
                .raw_write(decoded.rd.into(), (v as i16) as u32);
        }
        LoadInst::Lw => {
            let v: u32 = match state.bus.read_32_fast(addr as usize) {
                Some(v) => v,
                None => state
                    .bus
                    .read_32_slow_err(addr as usize)
                    .map_err(StateError::from)?,
            };
            state.reg.gpr.raw_write(decoded.rd.into(), v);
        }
        LoadInst::Lbu => {
            let v: u8 = match state.bus.read_8_fast(addr as usize) {
                Some(v) => v,
                None => state
                    .bus
                    .read_8_slow_err(addr as usize)
                    .map_err(StateError::from)?,
            };
            state.reg.gpr.raw_write(decoded.rd.into(), v as u32);
        }
        LoadInst::Lhu => {
            let v: u16 = match state.bus.read_16_fast(addr as usize) {
                Some(v) => v,
                None => state
                    .bus
                    .read_16_slow_err(addr as usize)
                    .map_err(StateError::from)?,
            };
            state.reg.gpr.raw_write(decoded.rd.into(), v as u32);
        }
    }
    Ok(pc.wrapping_add(4))
}
