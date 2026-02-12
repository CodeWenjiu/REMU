use remu_types::isa::reg::RegAccess;

use crate::riscv::inst::{funct3, funct7, imm_i, rd, rs1, DecodedInst, Inst};

pub(crate) const OPCODE: u32 = 0b001_0011;
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

#[derive(Clone, Copy, Debug)]
pub(crate) enum OpImmInst {
    Addi,
    Slli,
    Slti,
    Sltiu,
    Xori,
    Srli,
    Srai,
    Ori,
    Andi,
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);
    let f7 = funct7(inst);
    let rd = rd(inst);
    let rs1 = rs1(inst);
    let imm = imm_i(inst);
    let op = match f3 {
        func3::ADDI => OpImmInst::Addi,
        func3::SLLI => OpImmInst::Slli,
        func3::SLTI => OpImmInst::Slti,
        func3::SLTIU => OpImmInst::Sltiu,
        func3::XORI => OpImmInst::Xori,
        func3::SRI => match f7 {
            func7::NORMAL => OpImmInst::Srli,
            func7::ALT => OpImmInst::Srai,
            _ => return DecodedInst::default(),
        },
        func3::ORI => OpImmInst::Ori,
        func3::ANDI => OpImmInst::Andi,
        _ => return DecodedInst::default(),
    };
    DecodedInst {
        rd,
        rs1,
        rs2: 0,
        imm,
        inst: Inst::OpImm(op),
    }
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    let state = ctx.state_mut();
    let Inst::OpImm(op) = decoded.inst else { unreachable!() };
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let imm_val = decoded.imm;
    let value: u32 = match op {
        OpImmInst::Addi => rs1_val.wrapping_add(imm_val),
        OpImmInst::Slli => rs1_val.wrapping_shl(imm_val & 0x1F),
        OpImmInst::Slti => {
            if (rs1_val as i32) < (imm_val as i32) {
                1
            } else {
                0
            }
        }
        OpImmInst::Sltiu => if rs1_val < imm_val { 1 } else { 0 },
        OpImmInst::Xori => rs1_val ^ imm_val,
        OpImmInst::Srli => rs1_val.wrapping_shr(imm_val & 0x1F),
        OpImmInst::Srai => ((rs1_val as i32).wrapping_shr(imm_val & 0x1F)) as u32,
        OpImmInst::Ori => rs1_val | imm_val,
        OpImmInst::Andi => rs1_val & imm_val,
    };
    state.reg.gpr.raw_write(decoded.rd.into(), value);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}
