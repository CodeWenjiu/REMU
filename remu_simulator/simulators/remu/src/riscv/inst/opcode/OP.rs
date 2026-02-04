use std::marker::PhantomData;

use remu_types::isa::{reg::RegAccess, RvIsa};

use crate::riscv::inst::{funct3, funct7, rd, rs1, rs2, DecodedInst};

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
        handler!($name, state, inst, {
            let $rs1_val = state.reg.gpr.raw_read(inst.rs1.into());
            let $rs2_val = state.reg.gpr.raw_read(inst.rs2.into());
            let value: u32 = $value;
            state.reg.gpr.raw_write(inst.rd.into(), value);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        });
    };
}

op_op!(add, |rs1, rs2| rs1.wrapping_add(rs2));
op_op!(sub, |rs1, rs2| rs1.wrapping_sub(rs2));
op_op!(sll, |rs1, rs2| rs1.wrapping_shl(rs2 & 0x1F));
op_op!(slt, |rs1, rs2| if (rs1 as i32) < (rs2 as i32) {
    1
} else {
    0
});
op_op!(sltu, |rs1, rs2| if rs1 < rs2 { 1 } else { 0 });
op_op!(xor, |rs1, rs2| rs1 ^ rs2);
op_op!(or, |rs1, rs2| rs1 | rs2);
op_op!(and, |rs1, rs2| rs1 & rs2);
op_op!(srl, |rs1, rs2| rs1.wrapping_shr(rs2 & 0x1F));
op_op!(sra, |rs1, rs2| ((rs1 as i32).wrapping_shr(rs2 & 0x1F))
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

define_decode!(inst, {
    let f3 = funct3(inst);
    let f7 = funct7(inst);

    let rd = rd(inst);
    let rs1 = rs1(inst);
    let rs2 = rs2(inst);

    match (f3, f7) {
        (func3::ADD, func7::NORMAL) => DecodedInst::<P> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: add::<P>,
            _marker: PhantomData,
        },
        (func3::ADD, func7::ALT) => DecodedInst::<P> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: sub::<P>,
            _marker: PhantomData,
        },
        (func3::SLL, func7::NORMAL) => DecodedInst::<P> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: sll::<P>,
            _marker: PhantomData,
        },
        (func3::SLT, func7::NORMAL) => DecodedInst::<P> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: slt::<P>,
            _marker: PhantomData,
        },
        (func3::SLTU, func7::NORMAL) => DecodedInst::<P> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: sltu::<P>,
            _marker: PhantomData,
        },
        (func3::XOR, func7::NORMAL) => DecodedInst::<P> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: xor::<P>,
            _marker: PhantomData,
        },
        (func3::OR, func7::NORMAL) => DecodedInst::<P> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: or::<P>,
            _marker: PhantomData,
        },
        (func3::SR, func7::NORMAL) => DecodedInst::<P> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: srl::<P>,
            _marker: PhantomData,
        },
        (func3::SR, func7::ALT) => DecodedInst::<P> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: sra::<P>,
            _marker: PhantomData,
        },
        (func3::AND, func7::NORMAL) => DecodedInst::<P> {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: and::<P>,
            _marker: PhantomData,
        },

        (f3, func7::MAD) if P::ISA::HAS_M => match f3 {
            func3::MUL => DecodedInst::<P> {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: mul::<P>,
                _marker: PhantomData,
            },
            func3::MULH => DecodedInst::<P> {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: mulh::<P>,
                _marker: PhantomData,
            },
            func3::MULHSU => DecodedInst::<P> {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: mulhsu::<P>,
                _marker: PhantomData,
            },
            func3::MULHU => DecodedInst::<P> {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: mulhu::<P>,
                _marker: PhantomData,
            },
            func3::DIV => DecodedInst::<P> {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: div::<P>,
                _marker: PhantomData,
            },
            func3::DIVU => DecodedInst::<P> {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: divu::<P>,
                _marker: PhantomData,
            },
            func3::REM => DecodedInst::<P> {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: rem::<P>,
                _marker: PhantomData,
            },
            func3::REMU => DecodedInst::<P> {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: remu::<P>,
                _marker: PhantomData,
            },
            _ => DecodedInst::<P>::default(),
        },
        _ => DecodedInst::<P>::default(),
    }
});
