use std::marker::PhantomData;

use remu_types::{isa::reg::RegAccess, Xlen};

use crate::riscv::inst::{funct3, imm_b, rs1, rs2, DecodedInst};

pub(crate) const OPCODE: u32 = 0b110_0011;

pub(crate) const INSTRUCTION_MIX: u32 = 140;

mod func3 {
    pub const BEQ: u32 = 0b000;
    pub const BNE: u32 = 0b001;
    pub const BLT: u32 = 0b100;
    pub const BGE: u32 = 0b101;
    pub const BLTU: u32 = 0b110;
    pub const BGEU: u32 = 0b111;
}

macro_rules! branch_op {
    ($name:ident, |$a:ident, $b:ident| $cond:expr) => {
        handler!($name, state, inst, {
            let $a = state.reg.gpr.raw_read(inst.rs1.into());
            let $b = state.reg.gpr.raw_read(inst.rs2.into());
            if $cond {
                state.reg.pc = state.reg.pc.wrapping_add(inst.imm);
            } else {
                state.reg.pc = state.reg.pc.wrapping_add(4);
            }
            Ok(())
        });
    };
}

branch_op!(beq, |rs1, rs2| rs1 == rs2);
branch_op!(bne, |rs1, rs2| rs1 != rs2);
branch_op!(blt, |rs1, rs2| (rs1.to_signed()) < (rs2.to_signed()));
branch_op!(bge, |rs1, rs2| (rs1.to_signed()) >= (rs2.to_signed()));
branch_op!(bltu, |rs1, rs2| rs1 < rs2);
branch_op!(bgeu, |rs1, rs2| rs1 >= rs2);

define_decode!(inst, {
    let f3 = funct3(inst);

    let rs1 = rs1(inst);
    let rs2 = rs2(inst);
    let imm = imm_b(inst);

    match f3 {
        func3::BEQ => DecodedInst::<P> {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: beq::<P>,
            _marker: PhantomData,
        },
        func3::BNE => DecodedInst::<P> {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: bne::<P>,
            _marker: PhantomData,
        },
        func3::BLT => DecodedInst::<P> {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: blt::<P>,
            _marker: PhantomData,
        },
        func3::BGE => DecodedInst::<P> {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: bge::<P>,
            _marker: PhantomData,
        },
        func3::BLTU => DecodedInst::<P> {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: bltu::<P>,
            _marker: PhantomData,
        },
        func3::BGEU => DecodedInst::<P> {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: bgeu::<P>,
            _marker: PhantomData,
        },
        _ => DecodedInst::<P>::default(),
    }
});
