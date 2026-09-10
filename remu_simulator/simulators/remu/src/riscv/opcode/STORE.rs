use remu_isa::isa::reg::RegAccess;
use remu_state::StateError;

use crate::riscv::{DecodedInst, Inst, funct3, imm_s, rs1, rs2};

#[allow(dead_code)]
pub(crate) const OPCODE: u32 = 0b010_0011;
#[allow(dead_code)]
pub(crate) const INSTRUCTION_MIX: u32 = 110;

mod func3 {
    pub(super) const SB: u32 = 0b000;
    pub(super) const SH: u32 = 0b001;
    pub(super) const SW: u32 = 0b010;
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);
    let store = match f3 {
        func3::SB => Inst::Sb,
        func3::SH => Inst::Sh,
        func3::SW => Inst::Sw,
        _ => return DecodedInst::default(),
    };
    DecodedInst {
        rd: 0,
        rs1: rs1(inst),
        rs2: rs2(inst),
        imm: imm_s(inst),
        inst: store,
    }
}

#[inline(always)]
pub(crate) fn execute_sb<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
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
    let v = state.reg.gpr.raw_read(decoded.rs2.into()) as u8;
    match state.bus.write_8_fast(addr as usize, v) {
        Some(()) => {}
        None => state
            .bus
            .write_8_slow_err(addr as usize, v)
            .map_err(StateError::from)?,
    }
    Ok(pc.wrapping_add(4))
}

#[inline(always)]
pub(crate) fn execute_sh<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
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
    let v = state.reg.gpr.raw_read(decoded.rs2.into()) as u16;
    match state.bus.write_16_fast(addr as usize, v) {
        Some(()) => {}
        None => state
            .bus
            .write_16_slow_err(addr as usize, v)
            .map_err(StateError::from)?,
    }
    Ok(pc.wrapping_add(4))
}

#[inline(always)]
pub(crate) fn execute_sw<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
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
    let v = state.reg.gpr.raw_read(decoded.rs2.into());
    match state.bus.write_32_fast(addr as usize, v) {
        Some(()) => {}
        None => state
            .bus
            .write_32_slow_err(addr as usize, v)
            .map_err(StateError::from)?,
    }
    Ok(pc.wrapping_add(4))
}
