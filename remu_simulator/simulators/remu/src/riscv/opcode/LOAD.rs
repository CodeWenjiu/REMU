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

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);
    let load = match f3 {
        func3::LB => Inst::Lb,
        func3::LH => Inst::Lh,
        func3::LW => Inst::Lw,
        func3::LBU => Inst::Lbu,
        func3::LHU => Inst::Lhu,
        _ => return DecodedInst::default(),
    };
    DecodedInst {
        rd: rd(inst),
        rs1: rs1(inst),
        rs2: 0,
        imm: imm_i(inst),
        inst: load,
    }
}

#[inline(always)]
pub(crate) fn execute_lb<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    let addr = state
        .reg
        .gpr
        .raw_read(decoded.rs1.into())
        .wrapping_add(decoded.imm);
    let v: u8 = match state.bus.read_8_fast(addr as usize) {
        Some(v) => v,
        None => state
            .bus
            .read_8_slow_err(addr as usize)
            .map_err(StateError::from)?,
    };
    state.reg.gpr.raw_write(decoded.rd.into(), (v as i8) as u32);
    Ok(pc.wrapping_add(4))
}

#[inline(always)]
pub(crate) fn execute_lh<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    let addr = state
        .reg
        .gpr
        .raw_read(decoded.rs1.into())
        .wrapping_add(decoded.imm);
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
    Ok(pc.wrapping_add(4))
}

#[inline(always)]
pub(crate) fn execute_lw<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    let addr = state
        .reg
        .gpr
        .raw_read(decoded.rs1.into())
        .wrapping_add(decoded.imm);
    let v: u32 = match state.bus.read_32_fast(addr as usize) {
        Some(v) => v,
        None => state
            .bus
            .read_32_slow_err(addr as usize)
            .map_err(StateError::from)?,
    };
    state.reg.gpr.raw_write(decoded.rd.into(), v);
    Ok(pc.wrapping_add(4))
}

#[inline(always)]
pub(crate) fn execute_lbu<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    let addr = state
        .reg
        .gpr
        .raw_read(decoded.rs1.into())
        .wrapping_add(decoded.imm);
    let v: u8 = match state.bus.read_8_fast(addr as usize) {
        Some(v) => v,
        None => state
            .bus
            .read_8_slow_err(addr as usize)
            .map_err(StateError::from)?,
    };
    state.reg.gpr.raw_write(decoded.rd.into(), v as u32);
    Ok(pc.wrapping_add(4))
}

#[inline(always)]
pub(crate) fn execute_lhu<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    let addr = state
        .reg
        .gpr
        .raw_read(decoded.rs1.into())
        .wrapping_add(decoded.imm);
    let v: u16 = match state.bus.read_16_fast(addr as usize) {
        Some(v) => v,
        None => state
            .bus
            .read_16_slow_err(addr as usize)
            .map_err(StateError::from)?,
    };
    state.reg.gpr.raw_write(decoded.rd.into(), v as u32);
    Ok(pc.wrapping_add(4))
}
