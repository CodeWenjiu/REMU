use remu_isa::isa::reg::RegAccess;
use remu_isa::{WordOps, Xlen};

use crate::riscv::{DecodedInst, Inst, funct3, funct7, imm_i, rd, rs1};

#[allow(dead_code)]
pub(crate) const OPCODE: u32 = 0b001_0011;
/// RV64 word-immediate ALU opcode (addiw/slliw/srliw/sraiw).
pub(crate) const OPCODE_W: u32 = 0b001_1011;
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
        func3::SRI => {
            // RV64: shamt bit 5 sits in funct7 bit 0; drop it so funct7 checks
            // stay width-independent (RV32: that bit is always 0).
            // f7 >> 1 yields 6 bits: SRLI = 0b0000_00, SRAI = 0b0100_00.
            match f7 >> 1 {
                0b0000_00 => Inst::Srli,
                0b0100_00 => Inst::Srai,
                _ => return DecodedInst::default(),
            }
        }
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
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
    value: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    state.reg.gpr.raw_write(decoded.rd.into(), value);
    Ok(pc.wrapping_add(
        <<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_u64(4),
    ))
}

#[inline(always)]
pub(crate) fn execute_addi<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    finish(
        ctx,
        decoded,
        pc,
        rs1_val.wrapping_add(
            <<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_imm(
                decoded.imm,
            ),
        ),
    )
}

#[inline(always)]
pub(crate) fn execute_slli<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let shamt = decoded.imm & (<<P::ISA as remu_isa::isa::RvIsa>::XLEN as Xlen>::BITS - 1);
    finish(ctx, decoded, pc, rs1_val.wrapping_shl(shamt))
}

#[inline(always)]
pub(crate) fn execute_slti<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let v =
        <<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_u64(u64::from(
            rs1_val.to_signed()
                < <<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_imm(
                    decoded.imm,
                )
                .to_signed(),
        ));
    finish(ctx, decoded, pc, v)
}

#[inline(always)]
pub(crate) fn execute_sltiu<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let v =
        <<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_u64(u64::from(
            rs1_val
                < <<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_imm(
                    decoded.imm,
                ),
        ));
    finish(ctx, decoded, pc, v)
}

#[inline(always)]
pub(crate) fn execute_xori<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    finish(
        ctx,
        decoded,
        pc,
        rs1_val
            ^ <<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_imm(
                decoded.imm,
            ),
    )
}

#[inline(always)]
pub(crate) fn execute_srli<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let shamt = decoded.imm & (<<P::ISA as remu_isa::isa::RvIsa>::XLEN as Xlen>::BITS - 1);
    finish(ctx, decoded, pc, rs1_val.wrapping_shr(shamt))
}

#[inline(always)]
pub(crate) fn execute_srai<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let shamt = decoded.imm & (<<P::ISA as remu_isa::isa::RvIsa>::XLEN as Xlen>::BITS - 1);
    let v = Xlen::from_signed(rs1_val.to_signed().wrapping_shr(shamt));
    finish(ctx, decoded, pc, v)
}

#[inline(always)]
pub(crate) fn execute_ori<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    finish(
        ctx,
        decoded,
        pc,
        rs1_val
            | <<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_imm(
                decoded.imm,
            ),
    )
}

#[inline(always)]
pub(crate) fn execute_andi<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    finish(
        ctx,
        decoded,
        pc,
        rs1_val
            & <<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_imm(
                decoded.imm,
            ),
    )
}

/// RV64 word-immediate decode (opcode 0x1b): ADDIW / SLLIW / SRLIW / SRAIW.
/// Shift amounts are 5 bits for the word variants.
#[inline(always)]
pub(crate) fn decode_w<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);
    let f7 = funct7(inst);
    let op = match f3 {
        func3::ADDI => Inst::Addiw,
        func3::SLLI => Inst::Slliw,
        func3::SRI => match f7 {
            func7::NORMAL => Inst::Srliw,
            func7::ALT => Inst::Sraiw,
            _ => return DecodedInst::default(),
        },
        _ => return DecodedInst::default(),
    };
    DecodedInst {
        rd: rd(inst),
        rs1: rs1(inst),
        rs2: 0,
        imm: imm_i(inst),
        inst: op,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use remu_isa::isa::extension_enum::{RV32I, RV64IM};
    use remu_state::StateFastProfile;

    fn check<P: remu_state::StatePolicy>(inst: u32, want: Inst) {
        let got = decode::<P>(inst).inst;
        assert!(
            core::mem::discriminant(&got) == core::mem::discriminant(&want),
            "inst 0x{inst:08x}: got {got:?}, want {want:?}"
        );
    }

    #[test]
    fn decode_shift_rv32() {
        type P32 = StateFastProfile<RV32I>;
        check::<P32>(0x00751593, Inst::Slli); // slli a1, a0, 7
        check::<P32>(0x00755593, Inst::Srli); // srli a1, a0, 7
        check::<P32>(0x40755593, Inst::Srai); // srai a1, a0, 7
    }

    #[test]
    fn decode_shift_rv64() {
        type P64 = StateFastProfile<RV64IM>;
        // shamt bit 5 (bit 25 of the inst) must not break SRLI/SRAI detection.
        check::<P64>(0x00751593, Inst::Slli);
        check::<P64>(0x00755593, Inst::Srli);
        check::<P64>(0x40755593, Inst::Srai);
        check::<P64>(0x42755593, Inst::Srai); // srai a1, a0, 39 (shamt bit5=1)
        check::<P64>(0x03f55593, Inst::Srli); // srli a1, a0, 63 (shamt bit5=1)
    }

    #[test]
    fn decode_w_variants() {
        type P64 = StateFastProfile<RV64IM>;
        let w = |inst: u32| decode_w::<P64>(inst).inst;
        let want = |v: &Inst| core::mem::discriminant(v);
        assert_eq!(want(&w(0x0005051b)), want(&Inst::Addiw)); // addiw a0, a0, 0
        assert_eq!(want(&w(0x0075159b)), want(&Inst::Slliw)); // slliw a1, a0, 7
        assert_eq!(want(&w(0x0075559b)), want(&Inst::Srliw)); // srliw a1, a0, 7
        assert_eq!(want(&w(0x4075559b)), want(&Inst::Sraiw)); // sraiw a1, a0, 7
    }
}

#[inline(always)]
pub(crate) fn execute_addiw<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    // 32-bit add, sign-extend to XLEN.
    finish(
        ctx,
        decoded,
        pc,
        <<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_imm(
            rs1_val.to_u32().wrapping_add(decoded.imm),
        ),
    )
}

#[inline(always)]
pub(crate) fn execute_slliw<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let shamt = decoded.imm & 0x1f;
    finish(
        ctx,
        decoded,
        pc,
        <<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_imm(
            rs1_val.to_u32().wrapping_shl(shamt),
        ),
    )
}

#[inline(always)]
pub(crate) fn execute_srliw<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let shamt = decoded.imm & 0x1f;
    finish(
        ctx,
        decoded,
        pc,
        <<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_imm(
            rs1_val.to_u32().wrapping_shr(shamt),
        ),
    )
}

#[inline(always)]
pub(crate) fn execute_sraiw<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let shamt = decoded.imm & 0x1f;
    finish(
        ctx,
        decoded,
        pc,
        <<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_imm(
            (rs1_val.to_u32() as i32).wrapping_shr(shamt) as u32,
        ),
    )
}
