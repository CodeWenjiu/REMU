use remu_isa::isa::{RvIsa, reg::RegAccess};
use remu_isa::{WordOps, Xlen};

use crate::riscv::{DecodedInst, Inst, funct3, funct7, rd, rs1, rs2};

#[allow(dead_code)]
pub(crate) const OPCODE: u32 = 0b011_0011;
#[allow(dead_code)]
pub(crate) const INSTRUCTION_MIX: u32 = 130;

mod func3 {
    pub(super) const ADD: u32 = 0b000;
    pub(super) const SLL: u32 = 0b001;
    pub(super) const SLT: u32 = 0b010;
    pub(super) const SLTU: u32 = 0b011;
    pub(super) const XOR: u32 = 0b100;
    pub(super) const SR: u32 = 0b101;
    pub(super) const OR: u32 = 0b110;
    pub(super) const AND: u32 = 0b111;
    pub(super) const MUL: u32 = 0b000;
    pub(super) const MULH: u32 = 0b001;
    pub(super) const MULHSU: u32 = 0b010;
    pub(super) const MULHU: u32 = 0b011;
    pub(super) const DIV: u32 = 0b100;
    pub(super) const DIVU: u32 = 0b101;
    pub(super) const REM: u32 = 0b110;
    pub(super) const REMU: u32 = 0b111;
}
mod func7 {
    pub(super) const NORMAL: u32 = 0b0000000;
    pub(super) const ALT: u32 = 0b0100000;
    pub(super) const MAD: u32 = 0b0000001;
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);
    let f7 = funct7(inst);
    let rd = rd(inst);
    let rs1 = rs1(inst);
    let rs2 = rs2(inst);
    let inst_kind = match f7 {
        func7::NORMAL => match f3 {
            func3::ADD => Inst::Add,
            func3::SLL => Inst::Sll,
            func3::SLT => Inst::Slt,
            func3::SLTU => Inst::Sltu,
            func3::XOR => Inst::Xor,
            func3::SR => Inst::Srl,
            func3::OR => Inst::Or,
            func3::AND => Inst::And,
            _ => return DecodedInst::default(),
        },
        func7::ALT => match f3 {
            func3::ADD => Inst::Sub,
            func3::SR => Inst::Sra,
            _ => return DecodedInst::default(),
        },
        func7::MAD if P::ISA::HAS_M => match f3 {
            func3::MUL => Inst::Mul,
            func3::MULH => Inst::Mulh,
            func3::MULHSU => Inst::Mulhsu,
            func3::MULHU => Inst::Mulhu,
            func3::DIV => Inst::Div,
            func3::DIVU => Inst::Divu,
            func3::REM => Inst::Rem,
            func3::REMU => Inst::Remu,
            _ => return DecodedInst::default(),
        },
        _ => return DecodedInst::default(),
    };
    DecodedInst {
        rd,
        rs1,
        rs2,
        imm: 0,
        inst: inst_kind,
    }
}

#[inline(always)]
fn bool_w<P: remu_state::StatePolicy>(
    b: bool,
) -> <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN {
    <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_u64(u64::from(b))
}

#[inline(always)]
fn shamt<P: remu_state::StatePolicy>(
    b: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> u32 {
    b.to_u32() & b.shamt_mask()
}

#[inline(always)]
fn op2<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
    f: impl FnOnce(
        <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
        <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
    ) -> <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let rs2_val = state.reg.gpr.raw_read(decoded.rs2.into());
    let value = f(rs1_val, rs2_val);
    let state = ctx.state_mut();
    state.reg.gpr.raw_write(decoded.rd.into(), value);
    Ok(pc.wrapping_add(<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_u64(4)))
}

#[inline(always)]
pub(crate) fn execute_add<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a.wrapping_add(b))
}

#[inline(always)]
pub(crate) fn execute_sub<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a.wrapping_sub(b))
}

#[inline(always)]
pub(crate) fn execute_sll<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a.wrapping_shl(shamt::<P>(b)))
}

#[inline(always)]
pub(crate) fn execute_slt<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        bool_w::<P>(a.to_signed() < b.to_signed())
    })
}

#[inline(always)]
pub(crate) fn execute_sltu<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| bool_w::<P>(a < b))
}

#[inline(always)]
pub(crate) fn execute_xor<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a ^ b)
}

#[inline(always)]
pub(crate) fn execute_srl<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a.wrapping_shr(shamt::<P>(b)))
}

#[inline(always)]
pub(crate) fn execute_sra<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_signed(
            a.to_signed().wrapping_shr(shamt::<P>(b)),
        )
    })
}

#[inline(always)]
pub(crate) fn execute_or<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a | b)
}

#[inline(always)]
pub(crate) fn execute_and<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a & b)
}

#[inline(always)]
pub(crate) fn execute_mul<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a.wrapping_mul(b))
}

/// mulh / mulhsu / mulhu: high `BITS` bits of the 2·BITS product.
/// Implemented with 128-bit intermediates so it is XLEN-agnostic (RV32 & RV64).
#[inline(always)]
fn high_bits<P: remu_state::StatePolicy>(
    prod: i128,
    mask: u32,
) -> <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN {
    <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_u64((prod >> mask) as u64)
}

#[inline(always)]
pub(crate) fn execute_mulh<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        let sa = a.to_signed().to_i128();
        let sb = b.to_signed().to_i128();
        high_bits::<P>(sa.wrapping_mul(sb), b.shamt_mask() + 1)
    })
}

#[inline(always)]
pub(crate) fn execute_mulhsu<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        let sa = a.to_signed().to_i128();
        let ub = b.to_u128();
        high_bits::<P>(sa.wrapping_mul(ub as i128), b.shamt_mask() + 1)
    })
}

#[inline(always)]
pub(crate) fn execute_mulhu<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        let ua = a.to_u128();
        let ub = b.to_u128();
        <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_u64(
            (ua.wrapping_mul(ub) >> (b.shamt_mask() + 1)) as u64,
        )
    })
}

#[inline(always)]
pub(crate) fn execute_div<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        let sa = a.to_signed().to_i128();
        let sb = b.to_signed().to_i128();
        if sb == 0 {
            <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_i64(-1)
        } else if sa == i128::MIN >> (128 - (b.shamt_mask() + 1)) && sb == -1 {
            <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_i64(sa as i64)
        } else {
            <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_i64(
                sa.wrapping_div(sb) as i64
            )
        }
    })
}

#[inline(always)]
pub(crate) fn execute_divu<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        if b == a.wrapping_sub(a) {
            <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_u64(u64::MAX)
        } else {
            a.wrapping_div(b)
        }
    })
}

#[inline(always)]
pub(crate) fn execute_rem<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        let sa = a.to_signed().to_i128();
        let sb = b.to_signed().to_i128();
        if sb == 0 {
            a
        } else if sa == i128::MIN >> (128 - (b.shamt_mask() + 1)) && sb == -1 {
            <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_u64(0)
        } else {
            <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_i64(
                sa.wrapping_rem(sb) as i64
            )
        }
    })
}

#[inline(always)]
pub(crate) fn execute_remu<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        if b == 0.into() { a } else { a.wrapping_rem(b) }
    })
}
