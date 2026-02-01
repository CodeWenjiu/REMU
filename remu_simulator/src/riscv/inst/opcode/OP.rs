use std::marker::PhantomData;

use remu_state::{State, bus::BusObserver};
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
        fn $name<I: RvIsa, O: BusObserver>(
            state: &mut State<I>,
            inst: &DecodedInst<I, O>,
            _obs: &mut O,
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
pub(crate) fn decode<I: RvIsa, O: BusObserver>(inst: u32) -> DecodedInst<I, O> {
    let f3 = funct3(inst);
    let f7 = funct7(inst);

    let rd = rd(inst);
    let rs1 = rs1(inst);
    let rs2 = rs2(inst);

    match (f3, f7) {
        (func3::ADD, func7::NORMAL) => DecodedInst::<I, O> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: addi::<I, O>,
            _marker: PhantomData,
        },
        (func3::SLL, func7::NORMAL) => DecodedInst::<I, O> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: slli::<I, O>,
            _marker: PhantomData,
        },
        (func3::SLT, func7::NORMAL) => DecodedInst::<I, O> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: slti::<I, O>,
            _marker: PhantomData,
        },
        (func3::SLTU, func7::NORMAL) => DecodedInst::<I, O> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: sltiu::<I, O>,
            _marker: PhantomData,
        },
        (func3::XOR, func7::NORMAL) => DecodedInst::<I, O> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: xori::<I, O>,
            _marker: PhantomData,
        },
        (func3::OR, func7::NORMAL) => DecodedInst::<I, O> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: ori::<I, O>,
            _marker: PhantomData,
        },
        (func3::SR, func7::NORMAL) => DecodedInst::<I, O> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: srli::<I, O>,
            _marker: PhantomData,
        },
        (func3::SR, func7::ALT) => DecodedInst::<I, O> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: srai::<I, O>,
            _marker: PhantomData,
        },
        (func3::AND, func7::NORMAL) => DecodedInst::<I, O> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: andi::<I, O>,
            _marker: PhantomData,
        },

        (f3, func7::MAD) if I::HAS_M => match f3 {
            func3::MUL => DecodedInst::<I, O> {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: mul::<I, O>,
                _marker: PhantomData,
            },
            func3::MULH => DecodedInst::<I, O> {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: mulh::<I, O>,
                _marker: PhantomData,
            },
            func3::MULHSU => DecodedInst::<I, O> {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: mulhsu::<I, O>,
                _marker: PhantomData,
            },
            func3::MULHU => DecodedInst::<I, O> {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: mulhu::<I, O>,
                _marker: PhantomData,
            },
            func3::DIV => DecodedInst::<I, O> {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: div::<I, O>,
                _marker: PhantomData,
            },
            func3::DIVU => DecodedInst::<I, O> {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: divu::<I, O>,
                _marker: PhantomData,
            },
            func3::REM => DecodedInst::<I, O> {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: rem::<I, O>,
                _marker: PhantomData,
            },
            func3::REMU => DecodedInst::<I, O> {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: remu::<I, O>,
                _marker: PhantomData,
            },
            _ => DecodedInst::<I, O>::default(),
        },
        _ => DecodedInst::<I, O>::default(),
    }
}
