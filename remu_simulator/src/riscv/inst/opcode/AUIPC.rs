use std::marker::PhantomData;

use remu_types::isa::reg::RegAccess;

use crate::riscv::inst::{imm_u, rd, DecodedInst};

pub(crate) const OPCODE: u32 = 0b001_0111;

pub(crate) const INSTRUCTION_MIX: u32 = 20;

handler!(auipc, state, inst, {
    let value: u32 = state.reg.pc.wrapping_add(inst.imm);
    state.reg.gpr.raw_write(inst.rd.into(), value);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
});

define_decode!(inst, {
    DecodedInst::<P> {
        rs1: 0,
        rs2: 0,
        rd: rd(inst),
        imm: imm_u(inst),
        _marker: PhantomData,
        handler: auipc::<P>,
    }
});
