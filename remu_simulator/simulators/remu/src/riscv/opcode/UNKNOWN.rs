use remu_isa::isa::reg::Mcause;
use remu_isa::{WordOps, Xlen};
use remu_state::{State, StatePolicy};

use crate::riscv::{DecodedInst, Inst};

/// Illegal-instruction trap (M-mode); shared by [`execute`] and vector `mstatus.VS` checks.
/// `pc` is the faulting instruction address; returns the new (trap-vector) PC.
#[inline(always)]
pub(crate) fn trap_illegal_instruction<P: StatePolicy>(
    state: &mut State<P>,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> <P::ISA as remu_isa::isa::RvIsa>::XLEN {
    state.reg.csr.mepc = pc.to_u32();
    state.reg.csr.mcause = Mcause::IllegalInstruction.to_u32();
    state.reg.csr.mtval = 0;
    state.reg.csr.mstatus_apply_trap_entry();
    <<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_u64(state.reg.csr.mtvec_base() as u64)
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
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    Ok(trap_illegal_instruction(ctx.state_mut(), pc))
}
