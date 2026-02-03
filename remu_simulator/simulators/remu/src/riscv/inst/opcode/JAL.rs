use std::marker::PhantomData;

use remu_types::isa::reg::RegAccess;

use crate::riscv::inst::{imm_j, rd, DecodedInst};

pub(crate) const OPCODE: u32 = 0b110_1111;

pub(crate) const INSTRUCTION_MIX: u32 = 30;

handler!(jal, state, inst, {
    let value: u32 = state.reg.pc.wrapping_add(4);
    state.reg.gpr.raw_write(inst.rd.into(), value);
    *state.reg.pc = state.reg.pc.wrapping_add(inst.imm);
    Ok(())
});

define_decode!(inst, {
    DecodedInst::<P> {
        rs1: 0,
        rs2: 0,
        rd: rd(inst),
        imm: imm_j(inst),
        handler: jal::<P>,
        _marker: PhantomData,
    }
});
