use crate::riscv::inst::{
    DecodedInst, Inst, funct3,
    opcode::OP_V::{OpCfgInst, OpIviInst, OpIvvInst, OpIvxInst, OpMvvInst, OpMvxInst, VInst},
    rd, rs1, rs2,
};

pub(crate) const OPCODE: u32 = 0b101_0111; // OP-V
pub(crate) const INSTRUCTION_MIX: u32 = 5;

mod func3 {
    pub(super) const OPCFG: u32 = 0b111;
    pub(super) const OPIVV: u32 = 0b000;
    pub(super) const OPMVV: u32 = 0b010;
    pub(super) const OPIVI: u32 = 0b011;
    pub(super) const OPIVX: u32 = 0b100;
    pub(super) const OPMVX: u32 = 0b110;
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);
    let f6 = funct6(inst);
    let t2 = top2(inst);

    match f3 {
        func3::OPCFG => match t2 {
            0b11 => DecodedInst {
                rd: rd(inst),
                rs1: 0,
                rs2: 0,
                imm: (inst >> 15) & 0x7FFF,
                inst: Inst::V(VInst::OpCfg(OpCfgInst::Vsetivli)),
            },
            0b00 | 0b01 => DecodedInst {
                rd: rd(inst),
                rs1: rs1(inst),
                rs2: 0,
                imm: (inst >> 15) & 0x7FFF,
                inst: Inst::V(VInst::OpCfg(OpCfgInst::Vsetvli)),
            },
            _ => return DecodedInst::default(),
        },
        func3::OPIVV => match f6 {
            0b000010 => DecodedInst {
                rd: rd(inst),
                rs1: rs1(inst),
                rs2: rs2(inst),
                imm: vm(inst) as u32,
                inst: Inst::V(VInst::OpIvv(OpIvvInst::Vsub_vv)),
            },
            0b001010 => DecodedInst {
                rd: rd(inst),
                rs1: rs1(inst),
                rs2: rs2(inst),
                imm: vm(inst) as u32,
                inst: Inst::V(VInst::OpIvv(OpIvvInst::Vor_vv)),
            },
            _ => return DecodedInst::default(),
        },
        func3::OPMVV => match f6 {
            0b000000 => DecodedInst {
                rd: rd(inst),
                rs1: rs1(inst),
                rs2: rs2(inst),
                imm: vm(inst) as u32,
                inst: Inst::V(VInst::OpMvv(OpMvvInst::Vredsum_vs)),
            },
            0b010100 => match rs2(inst) {
                0b00000 if vm(inst) == 1 => DecodedInst {
                    rd: rd(inst),
                    rs1: 0,
                    rs2: 0,
                    imm: 0,
                    inst: Inst::V(VInst::OpMvv(OpMvvInst::Vid_v)),
                },
                _ => return DecodedInst::default(),
            },
            0b010000 => match rs1(inst) {
                0b00000 => DecodedInst {
                    rd: rd(inst),
                    rs1: 0,
                    rs2: rs2(inst),
                    imm: 0,
                    inst: Inst::V(VInst::OpMvv(OpMvvInst::Vmv_x_s)),
                },
                0b10001 => DecodedInst {
                    rd: rd(inst),
                    rs1: 0,
                    rs2: rs2(inst),
                    imm: vm(inst) as u32,
                    inst: Inst::V(VInst::OpMvv(OpMvvInst::Vfirst_m)),
                },
                _ => return DecodedInst::default(),
            },
            // Vector extension family (Funct6 = 010010): zext even rs1, sext odd rs1
            0b010010 => match rs1(inst) {
                0b00100 => DecodedInst {
                    rd: rd(inst),
                    rs1: 0,
                    rs2: rs2(inst),
                    imm: vm(inst) as u32,
                    inst: Inst::V(VInst::OpMvv(OpMvvInst::Vzext_vf4)),
                },
                0b00101 => DecodedInst {
                    rd: rd(inst),
                    rs1: 0,
                    rs2: rs2(inst),
                    imm: vm(inst) as u32,
                    inst: Inst::V(VInst::OpMvv(OpMvvInst::Vsext_vf4)),
                },
                // 0b00010 vzext.vf8, 0b00011 vsext.vf8, 0b00110 vzext.vf2, 0b00111 vsext.vf2 reserved
                _ => return DecodedInst::default(),
            },
            0b011010 if vm(inst) == 1 => DecodedInst {
                rd: rd(inst),
                rs1: rs1(inst),
                rs2: rs2(inst),
                imm: 1,
                inst: Inst::V(VInst::OpMvv(OpMvvInst::Vmor_mm)),
            },
            0b101101 => DecodedInst {
                rd: rd(inst),
                rs1: rs1(inst),
                rs2: rs2(inst),
                imm: vm(inst) as u32,
                inst: Inst::V(VInst::OpMvv(OpMvvInst::Vmacc_vv)),
            },
            0b111000 => DecodedInst {
                rd: rd(inst),
                rs1: rs1(inst),
                rs2: rs2(inst),
                imm: vm(inst) as u32,
                inst: Inst::V(VInst::OpMvv(OpMvvInst::Vwmulu_vv)),
            },
            0b111101 => DecodedInst {
                rd: rd(inst),
                rs1: rs1(inst),
                rs2: rs2(inst),
                imm: vm(inst) as u32,
                inst: Inst::V(VInst::OpMvv(OpMvvInst::Vwmacc_vv)),
            },
            _ => return DecodedInst::default(),
        },
        func3::OPIVI => match f6 {
            0b010111 => {
                let simm5 = ((rs1(inst) as i32) << 27 >> 27) as u32;
                DecodedInst {
                    rd: rd(inst),
                    rs1: vm(inst) as u8,
                    rs2: rs2(inst),
                    imm: simm5,
                    inst: Inst::V(VInst::OpIvi(OpIviInst::Vmerge_vim)),
                }
            }
            0b011000 => DecodedInst {
                rd: rd(inst),
                rs1: 0,
                rs2: rs2(inst),
                imm: (rs1(inst) as u32 & 0x1F) | ((vm(inst) as u32) << 8),
                inst: Inst::V(VInst::OpIvi(OpIviInst::Vmseq_vi)),
            },
            0b011001 => DecodedInst {
                rd: rd(inst),
                rs1: 0,
                rs2: rs2(inst),
                imm: (rs1(inst) as u32 & 0x1F) | ((vm(inst) as u32) << 8),
                inst: Inst::V(VInst::OpIvi(OpIviInst::Vmsne_vi)),
            },
            0b100111 => DecodedInst {
                rd: rd(inst),
                rs1: 0,
                rs2: rs2(inst),
                imm: rs1(inst) as u32, // simm5: nr = imm + 1 (vmv1r/2r/4r/8r.v)
                inst: Inst::V(VInst::OpIvi(OpIviInst::VmvNr_v)),
            },
            0b000011 => {
                let simm5 = ((rs1(inst) as i32) << 27 >> 27) as u32;
                DecodedInst {
                    rd: rd(inst),
                    rs1: vm(inst) as u8,
                    rs2: rs2(inst),
                    imm: simm5,
                    inst: Inst::V(VInst::OpIvi(OpIviInst::Vrsub_vi)),
                }
            }
            0b000000 | 0b001010 => {
                let simm5 = ((rs1(inst) as i32) << 27 >> 27) as u32;
                DecodedInst {
                    rd: rd(inst),
                    rs1: vm(inst) as u8,
                    rs2: rs2(inst),
                    imm: simm5,
                    inst: Inst::V(VInst::OpIvi(OpIviInst::Vadd_vi)),
                }
            }
            0b001001 => {
                let simm5 = ((rs1(inst) as i32) << 27 >> 27) as u32;
                DecodedInst {
                    rd: rd(inst),
                    rs1: vm(inst) as u8,
                    rs2: rs2(inst),
                    imm: simm5,
                    inst: Inst::V(VInst::OpIvi(OpIviInst::Vand_vi)),
                }
            }
            0b001111 => DecodedInst {
                rd: rd(inst),
                rs1: 0,
                rs2: rs2(inst),
                imm: (rs1(inst) as u32 & 0x1F) | ((vm(inst) as u32) << 8),
                inst: Inst::V(VInst::OpIvi(OpIviInst::Vslidedown_vi)),
            },
            0b100101 => DecodedInst {
                rd: rd(inst),
                rs1: 0,
                rs2: rs2(inst),
                imm: (rs1(inst) as u32 & 0x1F) | ((vm(inst) as u32) << 8),
                inst: Inst::V(VInst::OpIvi(OpIviInst::Vsll_vi)),
            },
            0b101000 => DecodedInst {
                rd: rd(inst),
                rs1: 0,
                rs2: rs2(inst),
                imm: (rs1(inst) as u32 & 0x1F) | ((vm(inst) as u32) << 8),
                inst: Inst::V(VInst::OpIvi(OpIviInst::Vsrl_vi)),
            },
            _ => return DecodedInst::default(),
        },
        func3::OPIVX => match f6 {
            0b000000 => DecodedInst {
                rd: rd(inst),
                rs1: rs1(inst),
                rs2: rs2(inst),
                imm: vm(inst) as u32,
                inst: Inst::V(VInst::OpIvx(OpIvxInst::Vadd_vx)),
            },
            0b001001 => DecodedInst {
                rd: rd(inst),
                rs1: rs1(inst),
                rs2: rs2(inst),
                imm: vm(inst) as u32,
                inst: Inst::V(VInst::OpIvx(OpIvxInst::Vand_vx)),
            },
            0b010111 => DecodedInst {
                rd: rd(inst),
                rs1: rs1(inst),
                rs2: rs2(inst),
                imm: vm(inst) as u32,
                inst: Inst::V(VInst::OpIvx(OpIvxInst::Vmerge_vxm)),
            },
            0b011000 => DecodedInst {
                rd: rd(inst),
                rs1: rs1(inst),
                rs2: rs2(inst),
                imm: vm(inst) as u32,
                inst: Inst::V(VInst::OpIvx(OpIvxInst::Vmseq_vx)),
            },
            0b011011 => DecodedInst {
                rd: rd(inst),
                rs1: rs1(inst),
                rs2: rs2(inst),
                imm: vm(inst) as u32,
                inst: Inst::V(VInst::OpIvx(OpIvxInst::Vmslt_vx)),
            },
            _ => return DecodedInst::default(),
        },
        func3::OPMVX => match f6 {
            0b010000 if rs2(inst) == 0 => DecodedInst {
                rd: rd(inst),
                rs1: rs1(inst),
                rs2: 0,
                imm: 0,
                inst: Inst::V(VInst::OpMvx(OpMvxInst::Vmv_s_x)),
            },
            // vslide1down.vx vd, vs2, rs1 (Spike MATCH_VSLIDE1DOWN_VX; funct3=OP-MVX)
            0b001111 => DecodedInst {
                rd: rd(inst),
                rs1: rs1(inst),
                rs2: rs2(inst),
                imm: vm(inst) as u32,
                inst: Inst::V(VInst::OpMvx(OpMvxInst::Vslide1down_vx)),
            },
            0b111011 => DecodedInst {
                rd: rd(inst),
                rs1: rs1(inst),
                rs2: rs2(inst),
                imm: vm(inst) as u32,
                inst: Inst::V(VInst::OpMvx(OpMvxInst::Vwmul_vx)),
            },
            _ => return DecodedInst::default(),
        },
        _ => DecodedInst::default(),
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
