use remu_state::{State, bus::BusAccess};
use remu_types::Rv32Isa;

use crate::riscv::inst::{DecodedInst, SimulatorError, funct3, imm_s, rs1, rs2};

pub(crate) const OPCODE: u32 = 0b010_0011;

pub(crate) const INSTRUCTION_MIX: u32 = 110;

mod func3 {
    pub const SB: u32 = 0b000; // Store Byte
    pub const SH: u32 = 0b001; // Store Halfword
    pub const SW: u32 = 0b010; // Store Word
}

fn sb<I: Rv32Isa>(state: &mut State<I>, inst: &DecodedInst<I>) -> Result<(), SimulatorError> {
    let rs1 = state.reg.read_gpr(inst.rs1.into());
    let addr = rs1.wrapping_add(inst.imm);
    state
        .bus
        .write_8(addr as usize, state.reg.read_gpr(inst.rs2.into()) as u8)?;
    state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

fn sh<I: Rv32Isa>(state: &mut State<I>, inst: &DecodedInst<I>) -> Result<(), SimulatorError> {
    let rs1 = state.reg.read_gpr(inst.rs1.into());
    let addr = rs1.wrapping_add(inst.imm);
    state
        .bus
        .write_16(addr as usize, state.reg.read_gpr(inst.rs2.into()) as u16)?;
    state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

fn sw<I: Rv32Isa>(state: &mut State<I>, inst: &DecodedInst<I>) -> Result<(), SimulatorError> {
    let rs1 = state.reg.read_gpr(inst.rs1.into());
    let addr = rs1.wrapping_add(inst.imm);
    state
        .bus
        .write_32(addr as usize, state.reg.read_gpr(inst.rs2.into()))?;
    state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

#[inline(always)]
pub(crate) fn decode<I: Rv32Isa>(inst: u32) -> DecodedInst<I> {
    let f3 = funct3(inst);

    let rs1 = rs1(inst);
    let rs2 = rs2(inst);
    let imm = imm_s(inst);

    match f3 {
        func3::SB => DecodedInst {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: sb,
        },
        func3::SH => DecodedInst {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: sh,
        },
        func3::SW => DecodedInst {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: sw,
        },
        _ => DecodedInst::default(),
    }
}
