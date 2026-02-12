use remu_types::isa::reg::Mcause;

use crate::riscv::inst::{DecodedInst, Inst};

pub(crate) const OPCODE: u32 = 0b111_1111;
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
pub(crate) fn execute<P: remu_state::StatePolicy>(
    state: &mut remu_state::State<P>,
    _decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    let fault_pc = *state.reg.pc;
    state.reg.csr.mepc = fault_pc;
    state.reg.csr.mcause = Mcause::IllegalInstruction.to_u32();
    state.reg.csr.mtval = 0;
    state.reg.csr.mstatus_apply_trap_entry();
    *state.reg.pc = state.reg.csr.mtvec_base().into();
    Ok(())
}
