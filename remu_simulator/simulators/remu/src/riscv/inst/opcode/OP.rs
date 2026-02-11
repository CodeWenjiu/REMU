use std::marker::PhantomData;

use remu_types::isa::{reg::RegAccess, RvIsa};

use crate::riscv::inst::{funct3, funct7, rd, rs1, rs2, DecodedInst, Inst};

pub(crate) const OPCODE: u32 = 0b011_0011;
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

#[derive(Clone, Copy, Debug)]
pub(crate) enum OpInst {
    Add,
    Sub,
    Sll,
    Slt,
    Sltu,
    Xor,
    Srl,
    Sra,
    Or,
    And,
    Mul,
    Mulh,
    Mulhsu,
    Mulhu,
    Div,
    Divu,
    Rem,
    Remu,
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst<P> {
    let f3 = funct3(inst);
    let f7 = funct7(inst);
    let rd = rd(inst);
    let rs1 = rs1(inst);
    let rs2 = rs2(inst);
    let op = match (f3, f7) {
        (func3::ADD, func7::NORMAL) => OpInst::Add,
        (func3::ADD, func7::ALT) => OpInst::Sub,
        (func3::SLL, func7::NORMAL) => OpInst::Sll,
        (func3::SLT, func7::NORMAL) => OpInst::Slt,
        (func3::SLTU, func7::NORMAL) => OpInst::Sltu,
        (func3::XOR, func7::NORMAL) => OpInst::Xor,
        (func3::SR, func7::NORMAL) => OpInst::Srl,
        (func3::SR, func7::ALT) => OpInst::Sra,
        (func3::OR, func7::NORMAL) => OpInst::Or,
        (func3::AND, func7::NORMAL) => OpInst::And,
        (f3, func7::MAD) if P::ISA::HAS_M => match f3 {
            func3::MUL => OpInst::Mul,
            func3::MULH => OpInst::Mulh,
            func3::MULHSU => OpInst::Mulhsu,
            func3::MULHU => OpInst::Mulhu,
            func3::DIV => OpInst::Div,
            func3::DIVU => OpInst::Divu,
            func3::REM => OpInst::Rem,
            func3::REMU => OpInst::Remu,
            _ => return DecodedInst::default(),
        },
        _ => return DecodedInst::default(),
    };
    DecodedInst {
        rd,
        rs1,
        rs2,
        imm: 0,
        inst: Inst::Op(op),
        _marker: PhantomData,
    }
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy>(
    state: &mut remu_state::State<P>,
    decoded: &DecodedInst<P>,
) -> Result<(), remu_state::StateError> {
    let Inst::Op(op) = decoded.inst else { unreachable!() };
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let rs2_val = state.reg.gpr.raw_read(decoded.rs2.into());
    let value: u32 = match op {
        OpInst::Add => rs1_val.wrapping_add(rs2_val),
        OpInst::Sub => rs1_val.wrapping_sub(rs2_val),
        OpInst::Sll => rs1_val.wrapping_shl(rs2_val & 0x1F),
        OpInst::Slt => {
            if (rs1_val as i32) < (rs2_val as i32) {
                1
            } else {
                0
            }
        }
        OpInst::Sltu => if rs1_val < rs2_val { 1 } else { 0 },
        OpInst::Xor => rs1_val ^ rs2_val,
        OpInst::Srl => rs1_val.wrapping_shr(rs2_val & 0x1F),
        OpInst::Sra => ((rs1_val as i32).wrapping_shr(rs2_val & 0x1F)) as u32,
        OpInst::Or => rs1_val | rs2_val,
        OpInst::And => rs1_val & rs2_val,
        OpInst::Mul => rs1_val.wrapping_mul(rs2_val),
        OpInst::Mulh => (rs1_val as i64)
            .wrapping_mul(rs2_val as i64)
            .wrapping_shr(32) as u32,
        OpInst::Mulhsu => (rs1_val as i32 as i64)
            .wrapping_mul(rs2_val as u32 as i64)
            .wrapping_shr(32) as u32,
        OpInst::Mulhu => (rs1_val as u64)
            .wrapping_mul(rs2_val as u64)
            .wrapping_shr(32) as u32,
        OpInst::Div => {
            if rs2_val == 0 {
                0xFFFFFFFF
            } else {
                (rs1_val as i32).wrapping_div(rs2_val as i32) as u32
            }
        }
        OpInst::Divu => {
            if rs2_val == 0 {
                0xFFFFFFFF
            } else {
                rs1_val.wrapping_div(rs2_val)
            }
        }
        OpInst::Rem => (rs1_val as i32).wrapping_rem(rs2_val as i32) as u32,
        OpInst::Remu => {
            if rs2_val == 0 {
                rs1_val
            } else {
                rs1_val.wrapping_rem(rs2_val)
            }
        }
    };
    state.reg.gpr.raw_write(decoded.rd.into(), value);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}
