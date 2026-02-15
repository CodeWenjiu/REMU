use crate::riscv::inst::{DecodedInst, Inst, funct3};

pub(crate) const OPCODE: u32 = 0b000_1111;
pub(crate) const INSTRUCTION_MIX: u32 = 10;

const FENCE_I_FUNCT3: u32 = 0b001;

#[derive(Clone, Copy, Debug)]
pub(crate) enum MiscMemInst {
    Fence,
    FenceI,
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);
    let misc = if f3 == FENCE_I_FUNCT3 {
        MiscMemInst::FenceI
    } else {
        MiscMemInst::Fence
    };
    DecodedInst {
        rs1: 0,
        rs2: 0,
        rd: 0,
        imm: 0,
        inst: Inst::MiscMem(misc),
    }
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    let Inst::MiscMem(misc) = decoded.inst else {
        unreachable!()
    };
    if matches!(misc, MiscMemInst::FenceI) {
        ctx.flush_icache();
    }
    let state = ctx.state_mut();
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}
