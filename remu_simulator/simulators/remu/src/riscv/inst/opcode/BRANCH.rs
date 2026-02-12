use remu_types::{isa::reg::RegAccess, Xlen};

use crate::riscv::inst::{funct3, imm_b, rs1, rs2, DecodedInst, Inst};

pub(crate) const OPCODE: u32 = 0b110_0011;
pub(crate) const INSTRUCTION_MIX: u32 = 140;

mod func3 {
    pub(super) const BEQ: u32 = 0b000;
    pub(super) const BNE: u32 = 0b001;
    pub(super) const BLT: u32 = 0b100;
    pub(super) const BGE: u32 = 0b101;
    pub(super) const BLTU: u32 = 0b110;
    pub(super) const BGEU: u32 = 0b111;
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum BranchInst {
    Beq,
    Bne,
    Blt,
    Bge,
    Bltu,
    Bgeu,
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);
    let branch = match f3 {
        func3::BEQ => BranchInst::Beq,
        func3::BNE => BranchInst::Bne,
        func3::BLT => BranchInst::Blt,
        func3::BGE => BranchInst::Bge,
        func3::BLTU => BranchInst::Bltu,
        func3::BGEU => BranchInst::Bgeu,
        _ => return DecodedInst::default(),
    };
    DecodedInst {
        rd: 0,
        rs1: rs1(inst),
        rs2: rs2(inst),
        imm: imm_b(inst),
        inst: Inst::Branch(branch),
    }
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    let state = ctx.state_mut();
    let Inst::Branch(b) = decoded.inst else { unreachable!() };
    let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());
    let rs2_val = state.reg.gpr.raw_read(decoded.rs2.into());
    let take = match b {
        BranchInst::Beq => rs1_val == rs2_val,
        BranchInst::Bne => rs1_val != rs2_val,
        BranchInst::Blt => (rs1_val.to_signed()) < (rs2_val.to_signed()),
        BranchInst::Bge => (rs1_val.to_signed()) >= (rs2_val.to_signed()),
        BranchInst::Bltu => rs1_val < rs2_val,
        BranchInst::Bgeu => rs1_val >= rs2_val,
    };
    if take {
        *state.reg.pc = state.reg.pc.wrapping_add(decoded.imm);
    } else {
        *state.reg.pc = state.reg.pc.wrapping_add(4);
    }
    Ok(())
}
