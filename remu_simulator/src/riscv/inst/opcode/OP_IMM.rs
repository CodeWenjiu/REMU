use std::marker::PhantomData;

use remu_types::isa::reg::RegAccess;

use crate::riscv::inst::{funct3, funct7, imm_i, rd, rs1, DecodedInst};

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
        handler!($name, state, inst, {
            let $rs1_val = state.reg.gpr.raw_read(inst.rs1.into());
            let $imm_val = inst.imm;
            let value: u32 = $value;
            state.reg.gpr.raw_write(inst.rd.into(), value);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        });
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

define_decode!(inst, {
    let f3 = funct3(inst);

    let rd = rd(inst);
    let rs1 = rs1(inst);
    let imm = imm_i(inst);

    match f3 {
        func3::ADDI => DecodedInst::<P> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: addi::<P>,
            _marker: PhantomData,
        },
        func3::SLLI => DecodedInst::<P> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: slli::<P>,
            _marker: PhantomData,
        },
        func3::SLTI => DecodedInst::<P> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: slti::<P>,
            _marker: PhantomData,
        },
        func3::SLTIU => DecodedInst::<P> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: sltiu::<P>,
            _marker: PhantomData,
        },
        func3::XORI => DecodedInst::<P> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: xori::<P>,
            _marker: PhantomData,
        },
        func3::ORI => DecodedInst::<P> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: ori::<P>,
            _marker: PhantomData,
        },
        func3::SRI => match funct7(inst) {
            func7::NORMAL => DecodedInst::<P> {
                rd,
                rs1,
                rs2: 0,
                imm,

                handler: srli::<P>,
                _marker: PhantomData,
            },
            func7::ALT => DecodedInst::<P> {
                rd,
                rs1,
                rs2: 0,
                imm,

                handler: srai::<P>,
                _marker: PhantomData,
            },
            _ => DecodedInst::<P>::default(),
        },
        func3::ANDI => DecodedInst::<P> {
            rd,
            rs1,
            rs2: 0,
            imm,

            handler: andi::<P>,
            _marker: PhantomData,
        },
        _ => DecodedInst::<P>::default(),
    }
});
