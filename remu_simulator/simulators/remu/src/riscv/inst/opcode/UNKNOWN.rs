use std::marker::PhantomData;

use remu_state::StatePolicy;
use remu_types::isa::reg::Mcause;

use crate::riscv::inst::DecodedInst;

pub(crate) const OPCODE: u32 = 0b111_1111;

pub(crate) const INSTRUCTION_MIX: u32 = 2;

handler!(trap_unknown_instruction, state, inst, {
    let _ = inst;
    let fault_pc = *state.reg.pc;
    state.reg.csr.mepc = fault_pc;
    state.reg.csr.mcause = Mcause::IllegalInstruction.to_u32();
    state.reg.csr.mtval = 0;
    state.reg.csr.mstatus_apply_trap_entry();
    *state.reg.pc = state.reg.csr.mtvec_base().into();
    Ok(())
});

impl<P: StatePolicy> Default for DecodedInst<P> {
    fn default() -> Self {
        Self {
            rs1: 0,
            rs2: 0,
            rd: 0,
            imm: 0,
            handler: trap_unknown_instruction::<P>,
            _marker: PhantomData,
        }
    }
}
