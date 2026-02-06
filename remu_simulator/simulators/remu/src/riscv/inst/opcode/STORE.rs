use std::marker::PhantomData;

use remu_state::StateError;
use remu_types::isa::reg::RegAccess;

use crate::riscv::inst::{DecodedInst, funct3, imm_s, rs1, rs2};

pub(crate) const OPCODE: u32 = 0b010_0011;

pub(crate) const INSTRUCTION_MIX: u32 = 110;

mod func3 {
    pub const SB: u32 = 0b000;
    pub const SH: u32 = 0b001;
    pub const SW: u32 = 0b010;
}

handler!(sb, state, inst, {
    let rs1 = state.reg.gpr.raw_read(inst.rs1.into());
    let addr = rs1.wrapping_add(inst.imm);
    state
        .bus
        .write_8(addr as usize, state.reg.gpr.raw_read(inst.rs2.into()) as u8)
        .map_err(StateError::from)?;
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
});

handler!(sh, state, inst, {
    let rs1 = state.reg.gpr.raw_read(inst.rs1.into());
    let addr = rs1.wrapping_add(inst.imm);
    state
        .bus
        .write_16(
            addr as usize,
            state.reg.gpr.raw_read(inst.rs2.into()) as u16,
        )
        .map_err(StateError::from)?;
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
});

handler!(sw, state, inst, {
    let rs1 = state.reg.gpr.raw_read(inst.rs1.into());
    let addr = rs1.wrapping_add(inst.imm);
    state
        .bus
        .write_32(addr as usize, state.reg.gpr.raw_read(inst.rs2.into()))
        .map_err(StateError::from)?;
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
});

define_decode!(inst, {
    let f3 = funct3(inst);

    let rs1 = rs1(inst);
    let rs2 = rs2(inst);
    let imm = imm_s(inst);

    match f3 {
        func3::SB => DecodedInst::<P> {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: sb::<P>,
            _marker: PhantomData,
        },
        func3::SH => DecodedInst::<P> {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: sh::<P>,
            _marker: PhantomData,
        },
        func3::SW => DecodedInst::<P> {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: sw::<P>,
            _marker: PhantomData,
        },
        _ => DecodedInst::<P>::default(),
    }
});
