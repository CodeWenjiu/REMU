use remu_isa::isa::reg::RegAccess;
use remu_isa::{WordOps, Xlen};

use crate::riscv::{DecodedInst, Inst, funct3, imm_b, rs1, rs2};

#[allow(dead_code)]
pub(crate) const OPCODE: u32 = 0b110_0011;
#[allow(dead_code)]
pub(crate) const INSTRUCTION_MIX: u32 = 140;

mod func3 {
    pub(super) const BEQ: u32 = 0b000;
    pub(super) const BNE: u32 = 0b001;
    pub(super) const BLT: u32 = 0b100;
    pub(super) const BGE: u32 = 0b101;
    pub(super) const BLTU: u32 = 0b110;
    pub(super) const BGEU: u32 = 0b111;
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);
    let branch = match f3 {
        func3::BEQ => Inst::Beq,
        func3::BNE => Inst::Bne,
        func3::BLT => Inst::Blt,
        func3::BGE => Inst::Bge,
        func3::BLTU => Inst::Bltu,
        func3::BGEU => Inst::Bgeu,
        _ => return DecodedInst::default(),
    };
    DecodedInst {
        rd: 0,
        rs1: rs1(inst),
        rs2: rs2(inst),
        imm: imm_b(inst),
        inst: branch,
    }
}

#[inline(always)]
fn execute_cond<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
    take: impl FnOnce(
        <P::ISA as remu_isa::isa::RvIsa>::XLEN,
        <P::ISA as remu_isa::isa::RvIsa>::XLEN,
    ) -> bool,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let rs2_val = state.reg.gpr.raw_read(decoded.rs2.into());
    Ok(if take(rs1_val, rs2_val) {
        pc.wrapping_add(<<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_u64(decoded.imm as u64))
    } else {
        pc.wrapping_add(<<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_u64(4))
    })
}

#[inline(always)]
pub(crate) fn execute_beq<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    execute_cond(ctx, decoded, pc, |a, b| a == b)
}

#[inline(always)]
pub(crate) fn execute_bne<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    execute_cond(ctx, decoded, pc, |a, b| a != b)
}

#[inline(always)]
pub(crate) fn execute_blt<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    execute_cond(ctx, decoded, pc, |a, b| a.to_signed() < b.to_signed())
}

#[inline(always)]
pub(crate) fn execute_bge<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    execute_cond(ctx, decoded, pc, |a, b| a.to_signed() >= b.to_signed())
}

#[inline(always)]
pub(crate) fn execute_bltu<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    execute_cond(ctx, decoded, pc, |a, b| a < b)
}

#[inline(always)]
pub(crate) fn execute_bgeu<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    execute_cond(ctx, decoded, pc, |a, b| a >= b)
}
