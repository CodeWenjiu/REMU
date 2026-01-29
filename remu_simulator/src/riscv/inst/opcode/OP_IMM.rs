use std::marker::PhantomData;

use remu_state::{State, bus::BusObserver};
use remu_types::isa::{RvIsa, reg::RegAccess};

use crate::riscv::inst::{DecodedInst, SimulatorError, funct3, funct7, imm_i, rd, rs1};

pub(crate) const OPCODE: u32 = 0b001_0011;

pub(crate) const INSTRUCTION_MIX: u32 = 260;

mod func3 {
    pub const ADDI: u32 = 0b000;
    pub const SLLI: u32 = 0b001;
    pub const SLTI: u32 = 0b010;
    pub const SLTIU: u32 = 0b011;
    pub const XORI: u32 = 0b100;
    pub const SRI: u32 = 0b101;
    pub const ORI: u32 = 0b110;
    pub const ANDI: u32 = 0b111;
}

mod func7 {
    pub const NORMAL: u32 = 0b0000000;
    pub const ALT: u32 = 0b0100000;
}

macro_rules! imm_op {
    ($name:ident, |$rs1_val:ident, $imm_val:ident| $value:expr) => {
        fn $name<I: RvIsa, M: BusObserver>(
            state: &mut State<I>,
            inst: &DecodedInst<I, M>,
        ) -> Result<(), SimulatorError> {
            let $rs1_val = state.reg.gpr.raw_read(inst.rs1.into());
            let $imm_val = inst.imm;
            let value: u32 = $value;
            state.reg.gpr.raw_write(inst.rd.into(), value);
            state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
    };
}

imm_op!(addi, |rs1, imm| rs1.wrapping_add(imm));
imm_op!(slli, |rs1, imm| rs1.wrapping_shl(imm & 0x1F));
imm_op!(slti, |rs1, imm| if (rs1 as i32) < (imm as i32) {
    1
} else {
    0
});
imm_op!(sltiu, |rs1, imm| if rs1 < imm { 1 } else { 0 });
imm_op!(xori, |rs1, imm| rs1 ^ imm);
imm_op!(ori, |rs1, imm| rs1 | imm);
imm_op!(andi, |rs1, imm| rs1 & imm);
imm_op!(srli, |rs1, imm| rs1.wrapping_shr(imm & 0x1F));
imm_op!(srai, |rs1, imm| ((rs1 as i32).wrapping_shr(imm & 0x1F))
    as u32);

#[inline(always)]
pub(crate) fn decode<I: RvIsa, O: BusObserver>(inst: u32) -> DecodedInst<I, O> {
    let f3 = funct3(inst);

    let rd = rd(inst);
    let rs1 = rs1(inst);
    let imm = imm_i(inst);

    match f3 {
        func3::ADDI => DecodedInst::<I, O> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: addi::<I, O>,
            _marker: PhantomData,
        },
        func3::SLLI => DecodedInst::<I, O> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: slli::<I, O>,
            _marker: PhantomData,
        },
        func3::SLTI => DecodedInst::<I, O> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: slti::<I, O>,
            _marker: PhantomData,
        },
        func3::SLTIU => DecodedInst::<I, O> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: sltiu::<I, O>,
            _marker: PhantomData,
        },
        func3::XORI => DecodedInst::<I, O> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: xori::<I, O>,
            _marker: PhantomData,
        },
        func3::ORI => DecodedInst::<I, O> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: ori::<I, O>,
            _marker: PhantomData,
        },
        func3::SRI => match funct7(inst) {
            func7::NORMAL => DecodedInst::<I, O> {
                rd,
                rs1,
                rs2: 0,
                imm,

                handler: srli::<I, O>,
                _marker: PhantomData,
            },
            func7::ALT => DecodedInst::<I, O> {
                rd,
                rs1,
                rs2: 0,
                imm,

                handler: srai::<I, O>,
                _marker: PhantomData,
            },
            _ => DecodedInst::<I, O>::default(),
        },
        func3::ANDI => DecodedInst::<I, O> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: andi::<I, O>,
            _marker: PhantomData,
        },
        _ => DecodedInst::<I, O>::default(),
    }
}
