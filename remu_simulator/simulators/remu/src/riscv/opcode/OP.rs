use remu_isa::isa::{RvIsa, reg::RegAccess};
use remu_isa::{WordOps, Xlen};

use crate::riscv::{DecodedInst, Inst, funct3, funct7, rd, rs1, rs2};

#[allow(dead_code)]
pub(crate) const OPCODE: u32 = 0b011_0011;
/// RV64 word ALU opcode (addw/subw/sllw/srlw/sraw + M word variants).
pub(crate) const OPCODE_W: u32 = 0b011_1011;
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

#[inline(always)]
pub(crate) fn execute_mulh<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a.mulh(b))
}

#[inline(always)]
pub(crate) fn execute_mulhsu<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a.mulhsu(b))
}

#[inline(always)]
pub(crate) fn execute_mulhu<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a.mulhu(b))
}

#[inline(always)]
pub(crate) fn execute_div<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        let sa = a.to_signed();
        let sb = b.to_signed();
        let zero = sa.wrapping_sub(sa);
        if sb == zero {
            // div by zero: all bits one
            <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_u64(u64::MAX)
        } else {
            // wrapping_div on the native width (i32/i64): MIN / -1 already
            // yields MIN, matching the RISC-V overflow rule, with no wide
            // intermediate and no libcall.
            <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_signed(sa.wrapping_div(sb))
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
        let sa = a.to_signed();
        let sb = b.to_signed();
        if sb == sa.wrapping_sub(sa) {
            a
        } else {
            // wrapping_rem on the native width (i32/i64): MIN % -1 already
            // yields 0, matching the RISC-V overflow rule.
            <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_signed(sa.wrapping_rem(sb))
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

/// RV64 word decode (opcode 0x3b): ADDW/SUBW/SLLW/SRLW/SRAW, and with the M
/// extension MULW/DIVW/DIVUW/REMW/REMUW. Shift amounts are 5 bits.
#[inline(always)]
pub(crate) fn decode_w<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);
    let f7 = funct7(inst);
    // Dispatch by funct7 first, like the RV32 OP::decode: ADD=f3 0b000
    // collides with MUL=f3 0b000, so the M arms must live in their own
    // (MAD) branch instead of shadowing the ADDW/SUBW arms.
    let op = match f7 {
        func7::NORMAL => match f3 {
            func3::ADD => Inst::Addw,
            func3::SLL => Inst::Sllw,
            func3::SR => Inst::Srlw,
            _ => return DecodedInst::default(),
        },
        func7::ALT => match f3 {
            func3::ADD => Inst::Subw,
            func3::SR => Inst::Sraw,
            _ => return DecodedInst::default(),
        },
        // RV64W M funct3 is NOT the RV32 M table: MULW=000, DIVW=001,
        // DIVUW=101, REMW=110, REMUW=111.
        func7::MAD if P::ISA::HAS_M => match f3 {
            func3::MUL => Inst::Mulw,
            func3::MULH => Inst::Divw,
            func3::DIVU => Inst::Divuw,
            func3::REM => Inst::Remw,
            func3::REMU => Inst::Remuw,
            _ => return DecodedInst::default(),
        },
        _ => return DecodedInst::default(),
    };
    DecodedInst {
        rd: rd(inst),
        rs1: rs1(inst),
        rs2: rs2(inst),
        imm: 0,
        inst: op,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use remu_isa::isa::extension_enum::RV64IM;
    use remu_state::StateFastProfile;

    type P = StateFastProfile<RV64IM>;

    fn check(inst: u32, want: Inst) {
        let got = decode_w::<P>(inst).inst;
        assert!(
            core::mem::discriminant(&got) == core::mem::discriminant(&want),
            "inst 0x{inst:08x}: got {got:?}, want {want:?}"
        );
    }

    #[test]
    fn decode_w_basic() {
        check(0x0005053b, Inst::Addw); // addw a0, a0, zero
        check(0x407505bb, Inst::Subw); // subw a1, a0, t2
        check(0x005515bb, Inst::Sllw); // sllw a1, a0, t0
        check(0x006555bb, Inst::Srlw); // srlw a1, a0, t1
        check(0x407555bb, Inst::Sraw); // sraw a1, a0, t2
    }

    #[test]
    fn decode_w_m_ext() {
        check(0x02a7053b, Inst::Mulw); // mulw a0, a4, a0 (funct3=000)
        check(0x02a7153b, Inst::Divw); // divw a0, a4, a0 (funct3=001)
        check(0x02a7553b, Inst::Divuw); // divuw a0, a4, a0 (funct3=101)
        check(0x02a7653b, Inst::Remw); // remw a0, a4, a0 (funct3=110)
        check(0x02a7753b, Inst::Remuw); // remuw a0, a4, a0 (funct3=111)
    }

    #[test]
    fn decode_w_reg_fields() {
        let d = decode_w::<P>(0x02a7053b);
        assert_eq!(d.rd, 10); // a0
        assert_eq!(d.rs1, 14); // a4
        assert_eq!(d.rs2, 10); // a0
    }

    #[test]
    fn decode_w_illegal() {
        // funct3=010 (SLT) has no W variant
        assert_eq!(
            core::mem::discriminant(&decode_w::<P>(0x00a7253b).inst),
            core::mem::discriminant(&Inst::Unknown)
        );
        // funct7=0b1100000 is a reserved (non-NORMAL/ALT/MAD) value
        assert_eq!(
            core::mem::discriminant(&decode_w::<P>(0xc0a7053b).inst),
            core::mem::discriminant(&Inst::Unknown)
        );
    }
}

#[inline(always)]
pub(crate) fn execute_addw<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_imm(
            a.to_u32().wrapping_add(b.to_u32()),
        )
    })
}

#[inline(always)]
pub(crate) fn execute_subw<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_imm(
            a.to_u32().wrapping_sub(b.to_u32()),
        )
    })
}

#[inline(always)]
pub(crate) fn execute_sllw<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_imm(
            a.to_u32().wrapping_shl(b.to_u32() & 0x1f),
        )
    })
}

#[inline(always)]
pub(crate) fn execute_srlw<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_imm(
            a.to_u32().wrapping_shr(b.to_u32() & 0x1f),
        )
    })
}

#[inline(always)]
pub(crate) fn execute_sraw<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_imm(
            (a.to_u32() as i32).wrapping_shr(b.to_u32() & 0x1f) as u32,
        )
    })
}

#[inline(always)]
pub(crate) fn execute_mulw<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_imm(
            a.to_u32().wrapping_mul(b.to_u32()),
        )
    })
}

#[inline(always)]
pub(crate) fn execute_divw<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        let (a, b) = (a.to_u32(), b.to_u32());
        if b == 0 {
            // div by zero: all bits one (sign-extended)
            <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_imm(u32::MAX)
        } else {
            <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_imm(
                (a as i32).wrapping_div(b as i32) as u32,
            )
        }
    })
}

#[inline(always)]
pub(crate) fn execute_divuw<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        let (a, b) = (a.to_u32(), b.to_u32());
        if b == 0 {
            <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_imm(u32::MAX)
        } else {
            <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_imm(a.wrapping_div(b))
        }
    })
}

#[inline(always)]
pub(crate) fn execute_remw<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        let (a, b) = (a.to_u32(), b.to_u32());
        if b == 0 {
            // rem by zero: dividend (sign-extended)
            <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_imm(a)
        } else {
            <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_imm(
                (a as i32).wrapping_rem(b as i32) as u32,
            )
        }
    })
}

#[inline(always)]
pub(crate) fn execute_remuw<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN,
) -> Result<<<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        let (a, b) = (a.to_u32(), b.to_u32());
        if b == 0 {
            <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_imm(a)
        } else {
            <<P as remu_state::StatePolicy>::ISA as RvIsa>::XLEN::from_imm(a.wrapping_rem(b))
        }
    })
}
