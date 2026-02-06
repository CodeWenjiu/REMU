use std::marker::PhantomData;

use remu_state::StateError;
use remu_types::isa::reg::RegAccess;

use crate::riscv::inst::{DecodedInst, funct3, imm_i, rd, rs1};

pub(crate) const OPCODE: u32 = 0b000_0011;

pub(crate) const INSTRUCTION_MIX: u32 = 220;

mod func3 {
    pub const LB: u32 = 0b000;
    pub const LH: u32 = 0b001;
    pub const LW: u32 = 0b010;
    pub const LBU: u32 = 0b100;
    pub const LHU: u32 = 0b101;
}

macro_rules! load_s {
    ($name:ident, $read_fn:ident, $u:ty, $i:ty) => {
        handler!($name, state, inst, {
            let rs1_val = state.reg.gpr.raw_read(inst.rs1.into());
            let addr = rs1_val.wrapping_add(inst.imm);
            let value: $u = state
                .bus
                .$read_fn(addr as usize)
                .map_err(StateError::from)?;
            state
                .reg
                .gpr
                .raw_write(inst.rd.into(), (value as $i) as u32);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        });
    };
}

macro_rules! load_u {
    ($name:ident, $read_fn:ident, $u:ty) => {
        handler!($name, state, inst, {
            let rs1_val = state.reg.gpr.raw_read(inst.rs1.into());
            let addr = rs1_val.wrapping_add(inst.imm);
            let value: $u = state
                .bus
                .$read_fn(addr as usize)
                .map_err(StateError::from)?;
            state.reg.gpr.raw_write(inst.rd.into(), value as u32);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        });
    };
}

load_s!(lb, read_8, u8, i8);
load_s!(lh, read_16, u16, i16);
load_u!(lbu, read_8, u8);
load_u!(lhu, read_16, u16);
load_u!(lw, read_32, u32);

define_decode!(inst, {
    let f3 = funct3(inst);

    let rs1 = rs1(inst);
    let rd = rd(inst);
    let imm = imm_i(inst);

    match f3 {
        func3::LB => DecodedInst::<P> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: lb::<P>,
            _marker: PhantomData,
        },
        func3::LH => DecodedInst::<P> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: lh::<P>,
            _marker: PhantomData,
        },
        func3::LBU => DecodedInst::<P> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: lbu::<P>,
            _marker: PhantomData,
        },
        func3::LHU => DecodedInst::<P> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: lhu::<P>,
            _marker: PhantomData,
        },
        func3::LW => DecodedInst::<P> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: lw::<P>,
            _marker: PhantomData,
        },
        _ => DecodedInst::<P>::default(),
    }
});
