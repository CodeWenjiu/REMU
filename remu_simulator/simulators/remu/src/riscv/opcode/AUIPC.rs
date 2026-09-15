use remu_isa::isa::reg::RegAccess;
use remu_isa::{Xlen, WordOps};

use crate::riscv::{DecodedInst, Inst, imm_u, rd};

#[allow(dead_code)]
pub(crate) const OPCODE: u32 = 0b001_0111;
#[allow(dead_code)]
pub(crate) const INSTRUCTION_MIX: u32 = 20;

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    DecodedInst {
        rs1: 0,
        rs2: 0,
        rd: rd(inst),
        imm: imm_u(inst),
        inst: Inst::Auipc,
    }
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as remu_isa::isa::RvIsa>::XLEN,
) -> Result<<P::ISA as remu_isa::isa::RvIsa>::XLEN, remu_state::StateError> {
    let state = ctx.state_mut();
    let value = pc.wrapping_add(<<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_imm(decoded.imm));
    state.reg.gpr.raw_write(decoded.rd.into(), value);
    Ok(pc.wrapping_add(<<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN::from_u64(4)))
}
