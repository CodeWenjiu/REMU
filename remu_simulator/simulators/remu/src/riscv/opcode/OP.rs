use remu_isa::isa::{RvIsa, reg::RegAccess};

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
fn op2<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
    f: impl FnOnce(u32, u32) -> u32,
) -> Result<u32, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let rs2_val = state.reg.gpr.raw_read(decoded.rs2.into());
    let value = f(rs1_val, rs2_val);
    let state = ctx.state_mut();
    state.reg.gpr.raw_write(decoded.rd.into(), value);
    Ok(pc.wrapping_add(4))
}

#[inline(always)]
pub(crate) fn execute_add<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a.wrapping_add(b))
}

#[inline(always)]
pub(crate) fn execute_sub<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a.wrapping_sub(b))
}

#[inline(always)]
pub(crate) fn execute_sll<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a.wrapping_shl(b & 0x1F))
}

#[inline(always)]
pub(crate) fn execute_slt<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| u32::from((a as i32) < (b as i32)))
}

#[inline(always)]
pub(crate) fn execute_sltu<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| u32::from(a < b))
}

#[inline(always)]
pub(crate) fn execute_xor<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a ^ b)
}

#[inline(always)]
pub(crate) fn execute_srl<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a.wrapping_shr(b & 0x1F))
}

#[inline(always)]
pub(crate) fn execute_sra<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        ((a as i32).wrapping_shr(b & 0x1F)) as u32
    })
}

#[inline(always)]
pub(crate) fn execute_or<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a | b)
}

#[inline(always)]
pub(crate) fn execute_and<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a & b)
}

#[inline(always)]
pub(crate) fn execute_mul<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| a.wrapping_mul(b))
}

#[inline(always)]
pub(crate) fn execute_mulh<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    // RV32 mulh: high XLEN bits of signed×signed product. Operands must be
    // sign-extended to i64; `u32 as i64` zero-extends and breaks negatives.
    op2(ctx, decoded, pc, |a, b| {
        (a as i32 as i64)
            .wrapping_mul(b as i32 as i64)
            .wrapping_shr(32) as u32
    })
}

#[inline(always)]
pub(crate) fn execute_mulhsu<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        (a as i32 as i64)
            .wrapping_mul(b as u32 as i64)
            .wrapping_shr(32) as u32
    })
}

#[inline(always)]
pub(crate) fn execute_mulhu<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        (a as u64).wrapping_mul(b as u64).wrapping_shr(32) as u32
    })
}

#[inline(always)]
pub(crate) fn execute_div<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        if b == 0 {
            0xFFFF_FFFF
        } else {
            (a as i32).wrapping_div(b as i32) as u32
        }
    })
}

#[inline(always)]
pub(crate) fn execute_divu<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        if b == 0 {
            0xFFFF_FFFF
        } else {
            a.wrapping_div(b)
        }
    })
}

#[inline(always)]
pub(crate) fn execute_rem<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    op2(ctx, decoded, pc, |a, b| {
        (a as i32).wrapping_rem(b as i32) as u32
    })
}

#[inline(always)]
pub(crate) fn execute_remu<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    op2(
        ctx,
        decoded,
        pc,
        |a, b| {
            if b == 0 { a } else { a.wrapping_rem(b) }
        },
    )
}
