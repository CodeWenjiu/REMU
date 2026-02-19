use crate::riscv::inst::{
    DecodedInst, Inst, funct3,
    opcode::OP_V::{OpCfgInst, OpIviInst, OpIvxInst, OpMvvInst, VInst},
    rd, rs1, rs2,
};

pub(crate) const OPCODE: u32 = 0b101_0111; // OP-V
pub(crate) const INSTRUCTION_MIX: u32 = 5;

mod func3 {
    pub(super) const OPCFG: u32 = 0b111;
    pub(super) const OPMVV: u32 = 0b010;
    pub(super) const OPIVI: u32 = 0b011;
    pub(super) const OPIVX: u32 = 0b100;
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);
    let f6 = funct6(inst);
    let t2 = top2(inst);

    let v_inst = match f3 {
        func3::OPCFG => match t2 {
            0b11 => VInst::OpCfg(OpCfgInst::Vsetivli),
            0b00 | 0b01 => VInst::OpCfg(OpCfgInst::Vsetvli),
            _ => return DecodedInst::default(),
        },
        func3::OPMVV => match f6 {
            0b000000 => VInst::OpMvv(OpMvvInst::Vredsum_vs),
            0b010100 => match rs2(inst) {
                0b00000 if vm(inst) == 1 => VInst::OpMvv(OpMvvInst::Vid_v),
                _ => return DecodedInst::default(),
            },
            0b010000 => match rs1(inst) {
                0b00000 => VInst::OpMvv(OpMvvInst::Vmv_x_s),
                0b10001 => VInst::OpMvv(OpMvvInst::Vfirst_m),
                _ => return DecodedInst::default(),
            },
            0b010111 => VInst::OpMvv(OpMvvInst::Vmerge_vim),
            0b011000 => VInst::OpMvv(OpMvvInst::Vmseq_vi),
            0b010010 => VInst::OpMvv(OpMvvInst::Vsext_vf4),
            _ => return DecodedInst::default(),
        },
        func3::OPIVI => match f6 {
            0b010111 => VInst::OpIvi(OpIviInst::Vmerge_vim),
            0b011000 => VInst::OpIvi(OpIviInst::Vmseq_vi),
            0b100111 => VInst::OpIvi(OpIviInst::Vmv1r_v),
            0b000011 => VInst::OpIvi(OpIviInst::Vrsub_vi),
            0b000000 => VInst::OpIvi(OpIviInst::Vadd_vi),
            0b001010 => VInst::OpIvi(OpIviInst::Vadd_vi), // vadd.vi alternate
            0b001111 => VInst::OpIvi(OpIviInst::Vslidedown_vi),
            _ => return DecodedInst::default(),
        },
        func3::OPIVX => match f6 {
            0b010111 => VInst::OpIvx(OpIvxInst::Vmerge_vxm),
            0b011011 => VInst::OpIvx(OpIvxInst::Vmslt_vx),
            0b001101 => {
                if rs2(inst) == 0 {
                    VInst::OpIvx(OpIvxInst::Vmv_s_x)
                } else {
                    return DecodedInst::default();
                }
            }
            _ => return DecodedInst::default(),
        },
        _ => return DecodedInst::default(),
    };

    match v_inst {
        VInst::OpCfg(OpCfgInst::Vsetivli) => DecodedInst {
            rd: rd(inst),
            rs1: 0,
            rs2: 0,
            imm: ((inst >> 15) & 0x7FFF),
            inst: Inst::V(v_inst),
        },
        VInst::OpCfg(OpCfgInst::Vsetvli) => DecodedInst {
            rd: rd(inst),
            rs1: rs1(inst),
            rs2: 0,
            imm: (inst >> 15) & 0x7FFF,
            inst: Inst::V(v_inst),
        },
        VInst::OpMvv(OpMvvInst::Vid_v) => DecodedInst {
            rd: rd(inst),
            rs1: 0,
            rs2: 0,
            imm: 0,
            inst: Inst::V(v_inst),
        },
        VInst::OpIvi(OpIviInst::Vrsub_vi) => {
            let simm5 = ((rs1(inst) as i32) << 27 >> 27) as u32;
            DecodedInst {
                rd: rd(inst),
                rs1: 0,
                rs2: rs2(inst),
                imm: simm5,
                inst: Inst::V(v_inst),
            }
        }
        VInst::OpIvi(OpIviInst::Vadd_vi) => {
            let simm5 = ((rs1(inst) as i32) << 27 >> 27) as u32;
            DecodedInst {
                rd: rd(inst),
                rs1: vm(inst) as u8,
                rs2: rs2(inst),
                imm: simm5,
                inst: Inst::V(v_inst),
            }
        }
        VInst::OpMvv(OpMvvInst::Vmerge_vim) | VInst::OpIvi(OpIviInst::Vmerge_vim) => {
            let simm5 = ((rs1(inst) as i32) << 27 >> 27) as u32;
            DecodedInst {
                rd: rd(inst),
                rs1: vm(inst) as u8,
                rs2: rs2(inst),
                imm: simm5,
                inst: Inst::V(v_inst),
            }
        }
        VInst::OpMvv(OpMvvInst::Vmseq_vi) | VInst::OpIvi(OpIviInst::Vmseq_vi) => {
            let simm5 = ((rs1(inst) as i32) << 27 >> 27) as u32;
            DecodedInst {
                rd: rd(inst),
                rs1: 0,
                rs2: rs2(inst),
                imm: simm5,
                inst: Inst::V(v_inst),
            }
        }
        VInst::OpIvx(OpIvxInst::Vmerge_vxm) => DecodedInst {
            rd: rd(inst),
            rs1: rs1(inst),
            rs2: rs2(inst),
            imm: 0,
            inst: Inst::V(v_inst),
        },
        VInst::OpMvv(OpMvvInst::Vsext_vf4) => DecodedInst {
            rd: rd(inst),
            rs1: 0,
            rs2: rs2(inst),
            imm: 0,
            inst: Inst::V(v_inst),
        },
        VInst::OpIvx(OpIvxInst::Vmv_s_x) => DecodedInst {
            rd: rd(inst),
            rs1: rs1(inst),
            rs2: 0,
            imm: 0,
            inst: Inst::V(v_inst),
        },
        VInst::OpIvi(OpIviInst::Vmv1r_v) => DecodedInst {
            rd: rd(inst),
            rs1: 0,
            rs2: rs2(inst),
            imm: 0,
            inst: Inst::V(v_inst),
        },
        VInst::OpIvx(OpIvxInst::Vmslt_vx) => DecodedInst {
            rd: rd(inst),
            rs1: rs1(inst),
            rs2: rs2(inst),
            imm: 0,
            inst: Inst::V(v_inst),
        },
        VInst::OpMvv(OpMvvInst::Vredsum_vs) => DecodedInst {
            rd: rd(inst),
            rs1: rs1(inst),
            rs2: rs2(inst),
            imm: 0,
            inst: Inst::V(v_inst),
        },
        VInst::OpMvv(OpMvvInst::Vmv_x_s) => DecodedInst {
            rd: rd(inst),
            rs1: 0,
            rs2: rs2(inst),
            imm: 0,
            inst: Inst::V(v_inst),
        },
        VInst::OpMvv(OpMvvInst::Vfirst_m) => DecodedInst {
            rd: rd(inst),
            rs1: 0,
            rs2: rs2(inst),
            imm: 0,
            inst: Inst::V(v_inst),
        },
        VInst::OpIvi(OpIviInst::Vslidedown_vi) => DecodedInst {
            rd: rd(inst),
            rs1: 0,
            rs2: rs2(inst),
            imm: rs1(inst) as u32,
            inst: Inst::V(v_inst),
        },
    }
}

#[inline(always)]
pub(crate) fn top2(inst: u32) -> u32 {
    (inst >> 30) & 0x3
}

#[inline(always)]
pub(crate) fn funct6(inst: u32) -> u32 {
    (inst >> 26) & 0x3F
}

#[inline(always)]
pub(crate) fn vm(inst: u32) -> u32 {
    (inst >> 25) & 1
}
