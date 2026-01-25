use remu_state::State;
use remu_types::{RvIsa, Xlen};

use crate::riscv::inst::{DecodedInst, SimulatorError, funct3, imm_b, rs1, rs2};

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
        fn $name<I: RvIsa>(
            state: &mut State<I>,
            inst: &DecodedInst<I>,
        ) -> Result<(), SimulatorError> {
            let $a = state.reg.read_gpr(inst.rs1.into());
            let $b = state.reg.read_gpr(inst.rs2.into());

            if $cond {
                state
                    .reg
                    .write_pc(state.reg.read_pc().wrapping_add(inst.imm));
            } else {
                state.reg.write_pc(state.reg.read_pc().wrapping_add(4));
            }

            Ok(())
        }
    };
}

branch_op!(beq, |rs1, rs2| rs1 == rs2);
branch_op!(bne, |rs1, rs2| rs1 != rs2);
branch_op!(blt, |rs1, rs2| (rs1.to_signed()) < (rs2.to_signed()));
branch_op!(bge, |rs1, rs2| (rs1.to_signed()) >= (rs2.to_signed()));
branch_op!(bltu, |rs1, rs2| rs1 < rs2);
branch_op!(bgeu, |rs1, rs2| rs1 >= rs2);

#[inline(always)]
pub(crate) fn decode<I: RvIsa>(inst: u32) -> DecodedInst<I> {
    let f3 = funct3(inst);

    let rs1 = rs1(inst);
    let rs2 = rs2(inst);
    let imm = imm_b(inst);

    match f3 {
        func3::BEQ => DecodedInst {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: beq,
        },
        func3::BNE => DecodedInst {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: bne,
        },
        func3::BLT => DecodedInst {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: blt,
        },
        func3::BGE => DecodedInst {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: bge,
        },
        func3::BLTU => DecodedInst {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: bltu,
        },
        func3::BGEU => DecodedInst {
            rd: 0,
            rs1,
            rs2,
            imm,

            handler: bgeu,
        },
        _ => DecodedInst::default(),
    }
}
