use remu_isa::isa::reg::RegAccess;
use remu_isa::{Xlen, WordOps};
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
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let addr = state
        .reg
        .gpr
        .raw_read(decoded.rs1.into())
        .wrapping_add(<<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_u64(decoded.imm as u64));
    let v = state.reg.gpr.raw_read(decoded.rs2.into()).to_u8();
    match state.bus.write_8_fast(addr.to_usize(), v) {
        Some(()) => {}
        None => state
            .bus
            .write_8_slow_err(addr.to_usize(), v)
            .map_err(StateError::from)?,
    }
    Ok(pc.wrapping_add(<<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_u64(4)))
}

#[inline(always)]
pub(crate) fn execute_sh<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let addr = state
        .reg
        .gpr
        .raw_read(decoded.rs1.into())
        .wrapping_add(<<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_u64(decoded.imm as u64));
    let v = state.reg.gpr.raw_read(decoded.rs2.into()).to_u16();
    match state.bus.write_16_fast(addr.to_usize(), v) {
        Some(()) => {}
        None => state
            .bus
            .write_16_slow_err(addr.to_usize(), v)
            .map_err(StateError::from)?,
    }
    Ok(pc.wrapping_add(<<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_u64(4)))
}

#[inline(always)]
pub(crate) fn execute_sw<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let addr = state
        .reg
        .gpr
        .raw_read(decoded.rs1.into())
        .wrapping_add(<<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_u64(decoded.imm as u64));
    let v = state.reg.gpr.raw_read(decoded.rs2.into());
    match state.bus.write_32_fast(addr.to_usize(), v.to_u32()) {
        Some(()) => {}
        None => state
            .bus
            .write_32_slow_err(addr.to_usize(), v.to_u32())
            .map_err(StateError::from)?,
    }
    Ok(pc.wrapping_add(<<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_u64(4)))
}
