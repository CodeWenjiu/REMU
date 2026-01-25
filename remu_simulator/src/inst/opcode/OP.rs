use remu_state::State;

use crate::inst::{DecodedInst, SimulatorError, funct3, funct7, rd, rs1, rs2};

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
}

mod func7 {
    pub const NORMAL: u32 = 0b0000000;
    pub const ALT: u32 = 0b0100000;
}

macro_rules! op_op {
    ($name:ident, |$rs1_val:ident, $rs2_val:ident| $value:expr) => {
        fn $name(state: &mut State, inst: &DecodedInst) -> Result<(), SimulatorError> {
            let $rs1_val = state.reg.read_gpr(inst.rs1.into());
            let $rs2_val = state.reg.read_gpr(inst.rs2.into());
            let value: u32 = $value;
            state.reg.write_gpr(inst.rd.into(), value);
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

#[inline(always)]
pub(crate) fn decode(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);

    let rd = rd(inst);
    let rs1 = rs1(inst);
    let rs2 = rs2(inst);

    match f3 {
        func3::ADD => DecodedInst {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: addi,
        },
        func3::SLL => DecodedInst {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: slli,
        },
        func3::SLT => DecodedInst {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: slti,
        },
        func3::SLTU => DecodedInst {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: sltiu,
        },
        func3::XOR => DecodedInst {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: xori,
        },
        func3::OR => DecodedInst {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: ori,
        },
        func3::SR => match funct7(inst) {
            func7::NORMAL => DecodedInst {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: srli,
            },
            func7::ALT => DecodedInst {
                rd,
                rs1,
                rs2,
                imm: 0,

                handler: srai,
            },
            _ => DecodedInst::default(),
        },
        func3::AND => DecodedInst {
            rd,
            rs1,
            rs2,
            imm: 0,

            handler: andi,
        },
        _ => DecodedInst::default(),
    }
}
