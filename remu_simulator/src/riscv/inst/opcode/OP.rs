use remu_state::State;
use remu_types::isa::{RvIsa, reg::RegAccess};

use crate::riscv::inst::{DecodedInst, SimulatorError, funct3, funct7, rd, rs1, rs2};

pub(crate) const OPCODE: u32 = 0b011_0011;

pub(crate) const INSTRUCTION_MIX: u32 = 130;

mod func3 {
    pub const ADD: u32 = 0b000;
    pub const SLL: u32 = 0b001;
    pub const SLT: u32 = 0b010;
    pub const SLTU: u32 = 0b011;
    pub const XOR: u32 = 0b100;
    pub const SR: u32 = 0b101;
    pub const OR: u32 = 0b110;
    pub const AND: u32 = 0b111;

    pub const MUL: u32 = 0b000;
    pub const MULH: u32 = 0b001;
    pub const MULHSU: u32 = 0b010;
    pub const MULHU: u32 = 0b011;
    pub const DIV: u32 = 0b100;
    pub const DIVU: u32 = 0b101;
    pub const REM: u32 = 0b110;
    pub const REMU: u32 = 0b111;
}

mod func7 {
    pub const NORMAL: u32 = 0b0000000;
    pub const ALT: u32 = 0b0100000;
    pub const MAD: u32 = 0b0000001;
}

macro_rules! op_op {
    ($name:ident, |$rs1_val:ident, $rs2_val:ident| $value:expr) => {
        fn $name<I: RvIsa>(
            state: &mut State<I>,
            inst: &DecodedInst<I>,
        ) -> Result<(), SimulatorError> {
            let $rs1_val = state.reg.gpr.raw_read(inst.rs1.into());
            let $rs2_val = state.reg.gpr.raw_read(inst.rs2.into());
            let value: u32 = $value;
            state.reg.gpr.raw_write(inst.rd.into(), value);
            state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
    };
}

op_op!(addi, |rs1, rs2| rs1.wrapping_add(rs2));
op_op!(slli, |rs1, rs2| rs1.wrapping_shl(rs2 & 0x1F));
op_op!(slti, |rs1, rs2| if (rs1 as i32) < (rs2 as i32) {
    1
} else {
    0
});
op_op!(sltiu, |rs1, rs2| if rs1 < rs2 { 1 } else { 0 });
op_op!(xori, |rs1, rs2| rs1 ^ rs2);
op_op!(ori, |rs1, rs2| rs1 | rs2);
op_op!(andi, |rs1, rs2| rs1 & rs2);
op_op!(srli, |rs1, rs2| rs1.wrapping_shr(rs2 & 0x1F));
op_op!(srai, |rs1, rs2| ((rs1 as i32).wrapping_shr(rs2 & 0x1F))
    as u32);

op_op!(mul, |rs1, rs2| rs1.wrapping_mul(rs2));
op_op!(
    mulh,
    |rs1, rs2| (rs1 as i64).wrapping_mul(rs2 as i64).wrapping_shr(32) as u32
);
op_op!(mulhsu, |rs1, rs2| (rs1 as i32 as i64)
    .wrapping_mul(rs2 as u32 as i64)
    .wrapping_shr(32) as u32);
op_op!(
    mulhu,
    |rs1, rs2| (rs1 as u64).wrapping_mul(rs2 as u64).wrapping_shr(32) as u32
);
op_op!(div, |rs1, rs2| if rs2 == 0 {
    0xFFFFFFFF
} else {
    (rs1 as i32).wrapping_div(rs2 as i32) as u32
});
op_op!(divu, |rs1, rs2| if rs2 == 0 {
    0xFFFFFFFF
} else {
    rs1.wrapping_div(rs2)
});
op_op!(rem, |rs1, rs2| (rs1 as i32).wrapping_rem(rs2 as i32) as u32);
op_op!(remu, |rs1, rs2| if rs2 == 0 {
    rs1
} else {
    rs1.wrapping_rem(rs2)
});

#[inline(always)]
pub(crate) fn decode<I: RvIsa>(inst: u32) -> DecodedInst<I> {
    let f3 = funct3(inst);
    let f7 = funct7(inst);

    let rd = rd(inst);
    let rs1 = rs1(inst);
    let rs2 = rs2(inst);

    match (f3, f7) {
        (func3::ADD, func7::NORMAL) => DecodedInst {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: addi,
        },
        (func3::SLL, func7::NORMAL) => DecodedInst {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: slli,
        },
        (func3::SLT, func7::NORMAL) => DecodedInst {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: slti,
        },
        (func3::SLTU, func7::NORMAL) => DecodedInst {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: sltiu,
        },
        (func3::XOR, func7::NORMAL) => DecodedInst {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: xori,
        },
        (func3::OR, func7::NORMAL) => DecodedInst {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: ori,
        },
        (func3::SR, func7::NORMAL) => DecodedInst {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: srli,
        },
        (func3::SR, func7::ALT) => DecodedInst {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: srai,
        },
        (func3::AND, func7::NORMAL) => DecodedInst {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: andi,
        },

        (f3, func7::MAD) if I::HAS_M => match f3 {
            func3::MUL => DecodedInst {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: mul,
            },
            func3::MULH => DecodedInst {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: mulh,
            },
            func3::MULHSU => DecodedInst {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: mulhsu,
            },
            func3::MULHU => DecodedInst {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: mulhu,
            },
            func3::DIV => DecodedInst {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: div,
            },
            func3::DIVU => DecodedInst {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: divu,
            },
            func3::REM => DecodedInst {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: rem,
            },
            func3::REMU => DecodedInst {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: remu,
            },
            _ => DecodedInst::default(),
        },
        _ => DecodedInst::default(),
    }
}
