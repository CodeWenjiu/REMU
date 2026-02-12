use crate::riscv::inst::{funct3, DecodedInst, Inst};

pub(crate) const OPCODE: u32 = 0b000_1111;
pub(crate) const INSTRUCTION_MIX: u32 = 10;

const FENCE_I_FUNCT3: u32 = 0b001;

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);
    let inst_kind = if f3 == FENCE_I_FUNCT3 {
        Inst::FenceI
    } else {
        Inst::Fence
    };
    DecodedInst {
        rs1: 0,
        rs2: 0,
        rd: 0,
        imm: 0,
        inst: inst_kind,
    }
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    if matches!(decoded.inst, Inst::FenceI) {
        ctx.flush_icache();
    }
    let state = ctx.state_mut();
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}
