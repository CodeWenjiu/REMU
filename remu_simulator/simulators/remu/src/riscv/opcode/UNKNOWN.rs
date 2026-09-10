use remu_isa::isa::reg::Mcause;
use remu_state::{State, StatePolicy};

use crate::riscv::{DecodedInst, Inst};

/// Illegal-instruction trap (M-mode); shared by [`execute`] and vector `mstatus.VS` checks.
/// `pc` is the faulting instruction address; returns the new (trap-vector) PC.
#[inline(always)]
pub(crate) fn trap_illegal_instruction<P: StatePolicy>(state: &mut State<P>, pc: u32) -> u32 {
    state.reg.csr.mepc = pc;
    state.reg.csr.mcause = Mcause::IllegalInstruction.to_u32();
    state.reg.csr.mtval = 0;
    state.reg.csr.mstatus_apply_trap_entry();
    state.reg.csr.mtvec_base().into()
}

#[allow(dead_code)]
pub(crate) const OPCODE: u32 = 0b111_1111;
#[allow(dead_code)]
pub(crate) const INSTRUCTION_MIX: u32 = 2;

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(_inst: u32) -> DecodedInst {
    DecodedInst {
        rs1: 0,
        rs2: 0,
        rd: 0,
        imm: 0,
        inst: Inst::Unknown,
    }
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    _decoded: &DecodedInst,
    pc: u32,
) -> Result<u32, remu_state::StateError> {
    Ok(trap_illegal_instruction(ctx.state_mut(), pc))
}
