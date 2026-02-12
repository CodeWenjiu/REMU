use std::hint::unreachable_unchecked;

use remu_types::isa::{RvIsa, reg::RegAccess};

use crate::riscv::inst::{DecodedInst, Inst, funct3, funct7, rd, rs1, rs2};

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

/// func7 == 0b0000000：Add, Sll, Slt, Sltu, Xor, Srl, Or, And
#[derive(Clone, Copy, Debug)]
pub(crate) enum OpInstF7_0 {
    Add,
    Sll,
    Slt,
    Sltu,
    Xor,
    Srl,
    Or,
    And,
}

/// func7 == 0b0100000：Sub, Sra
#[derive(Clone, Copy, Debug)]
pub(crate) enum OpInstF7Alt {
    Sub,
    Sra,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum OpInstM {
    Mul,
    Mulh,
    Mulhsu,
    Mulhu,
    Div,
    Divu,
    Rem,
    Remu,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum OpInst {
    F7_0(OpInstF7_0),
    F7Alt(OpInstF7Alt),
    M(OpInstM),
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);
    let f7 = funct7(inst);
    let rd = rd(inst);
    let rs1 = rs1(inst);
    let rs2 = rs2(inst);
    let op = match f7 {
        func7::NORMAL => OpInst::F7_0(match f3 {
            func3::ADD => OpInstF7_0::Add,
            func3::SLL => OpInstF7_0::Sll,
            func3::SLT => OpInstF7_0::Slt,
            func3::SLTU => OpInstF7_0::Sltu,
            func3::XOR => OpInstF7_0::Xor,
            func3::SR => OpInstF7_0::Srl,
            func3::OR => OpInstF7_0::Or,
            func3::AND => OpInstF7_0::And,
            _ => return DecodedInst::default(),
        }),
        func7::ALT => OpInst::F7Alt(match f3 {
            func3::ADD => OpInstF7Alt::Sub,
            func3::SR => OpInstF7Alt::Sra,
            _ => return DecodedInst::default(),
        }),
        func7::MAD if P::ISA::HAS_M => OpInst::M(match f3 {
            func3::MUL => OpInstM::Mul,
            func3::MULH => OpInstM::Mulh,
            func3::MULHSU => OpInstM::Mulhsu,
            func3::MULHU => OpInstM::Mulhu,
            func3::DIV => OpInstM::Div,
            func3::DIVU => OpInstM::Divu,
            func3::REM => OpInstM::Rem,
            func3::REMU => OpInstM::Remu,
            _ => return DecodedInst::default(),
        }),
        _ => return DecodedInst::default(),
    };
    DecodedInst {
        rd,
        rs1,
        rs2,
        imm: 0,
        inst: Inst::Op(op),
    }
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy>(
    state: &mut remu_state::State<P>,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    let Inst::Op(op) = decoded.inst else {
        unreachable!()
    };
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let rs2_val = state.reg.gpr.raw_read(decoded.rs2.into());
    let value: u32 = match op {
        OpInst::F7_0(r) => match r {
            OpInstF7_0::Add => rs1_val.wrapping_add(rs2_val),
            OpInstF7_0::Sll => rs1_val.wrapping_shl(rs2_val & 0x1F),
            OpInstF7_0::Slt => {
                if (rs1_val as i32) < (rs2_val as i32) {
                    1
                } else {
                    0
                }
            }
            OpInstF7_0::Sltu => {
                if rs1_val < rs2_val {
                    1
                } else {
                    0
                }
            }
            OpInstF7_0::Xor => rs1_val ^ rs2_val,
            OpInstF7_0::Srl => rs1_val.wrapping_shr(rs2_val & 0x1F),
            OpInstF7_0::Or => rs1_val | rs2_val,
            OpInstF7_0::And => rs1_val & rs2_val,
        },
        OpInst::F7Alt(a) => match a {
            OpInstF7Alt::Sub => rs1_val.wrapping_sub(rs2_val),
            OpInstF7Alt::Sra => ((rs1_val as i32).wrapping_shr(rs2_val & 0x1F)) as u32,
        },
        OpInst::M(m) => {
            if !P::ISA::HAS_M {
                unsafe { unreachable_unchecked() };
            }
            match m {
                OpInstM::Mul => rs1_val.wrapping_mul(rs2_val),
                OpInstM::Mulh => (rs1_val as i64)
                    .wrapping_mul(rs2_val as i64)
                    .wrapping_shr(32) as u32,
                OpInstM::Mulhsu => (rs1_val as i32 as i64)
                    .wrapping_mul(rs2_val as u32 as i64)
                    .wrapping_shr(32) as u32,
                OpInstM::Mulhu => (rs1_val as u64)
                    .wrapping_mul(rs2_val as u64)
                    .wrapping_shr(32) as u32,
                OpInstM::Div => {
                    if rs2_val == 0 {
                        0xFFFFFFFF
                    } else {
                        (rs1_val as i32).wrapping_div(rs2_val as i32) as u32
                    }
                }
                OpInstM::Divu => {
                    if rs2_val == 0 {
                        0xFFFFFFFF
                    } else {
                        rs1_val.wrapping_div(rs2_val)
                    }
                }
                OpInstM::Rem => (rs1_val as i32).wrapping_rem(rs2_val as i32) as u32,
                OpInstM::Remu => {
                    if rs2_val == 0 {
                        rs1_val
                    } else {
                        rs1_val.wrapping_rem(rs2_val)
                    }
                }
            }
        }
    };
    state.reg.gpr.raw_write(decoded.rd.into(), value);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}
