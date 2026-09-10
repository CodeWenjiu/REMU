use remu_isa::isa::reg::RegAccess;

use crate::riscv::{DecodedInst, Inst, funct3, funct7, imm_i, rd, rs1};

#[allow(dead_code)]
pub(crate) const OPCODE: u32 = 0b001_0011;
#[allow(dead_code)]
pub(crate) const INSTRUCTION_MIX: u32 = 260;

mod func3 {
    pub(super) const ADDI: u32 = 0b000;
    pub(super) const SLLI: u32 = 0b001;
    pub(super) const SLTI: u32 = 0b010;
    pub(super) const SLTIU: u32 = 0b011;
    pub(super) const XORI: u32 = 0b100;
    pub(super) const SRI: u32 = 0b101;
    pub(super) const ORI: u32 = 0b110;
    pub(super) const ANDI: u32 = 0b111;
}
mod func7 {
    pub(super) const NORMAL: u32 = 0b0000000;
    pub(super) const ALT: u32 = 0b0100000;
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);
    let f7 = funct7(inst);
    let rd = rd(inst);
    let rs1 = rs1(inst);
    let imm = imm_i(inst);
    let op = match f3 {
        func3::ADDI => Inst::Addi,
        func3::SLLI => Inst::Slli,
        func3::SLTI => Inst::Slti,
        func3::SLTIU => Inst::Sltiu,
        func3::XORI => Inst::Xori,
        func3::SRI => match f7 {
            func7::NORMAL => Inst::Srli,
            func7::ALT => Inst::Srai,
            _ => return DecodedInst::default(),
        },
        func3::ORI => Inst::Ori,
        func3::ANDI => Inst::Andi,
        _ => return DecodedInst::default(),
    };
    DecodedInst {
        rd,
        rs1,
        rs2: 0,
        imm,
        inst: op,
    }
}

#[inline(always)]
fn finish<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
    value: u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    state.reg.gpr.raw_write(decoded.rd.into(), value);
    Ok(pc.wrapping_add(4))
}

#[inline(always)]
pub(crate) fn execute_addi<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    finish(ctx, decoded, pc, rs1_val.wrapping_add(decoded.imm))
}

#[inline(always)]
pub(crate) fn execute_slli<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    finish(ctx, decoded, pc, rs1_val.wrapping_shl(decoded.imm & 0x1F))
}

#[inline(always)]
pub(crate) fn execute_slti<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let v = u32::from((rs1_val as i32) < (decoded.imm as i32));
    finish(ctx, decoded, pc, v)
}

#[inline(always)]
pub(crate) fn execute_sltiu<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let v = u32::from(rs1_val < decoded.imm);
    finish(ctx, decoded, pc, v)
}

#[inline(always)]
pub(crate) fn execute_xori<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    finish(ctx, decoded, pc, rs1_val ^ decoded.imm)
}

#[inline(always)]
pub(crate) fn execute_srli<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    finish(ctx, decoded, pc, rs1_val.wrapping_shr(decoded.imm & 0x1F))
}

#[inline(always)]
pub(crate) fn execute_srai<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let v = ((rs1_val as i32).wrapping_shr(decoded.imm & 0x1F)) as u32;
    finish(ctx, decoded, pc, v)
}

#[inline(always)]
pub(crate) fn execute_ori<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    finish(ctx, decoded, pc, rs1_val | decoded.imm)
}

#[inline(always)]
pub(crate) fn execute_andi<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    finish(ctx, decoded, pc, rs1_val & decoded.imm)
}
