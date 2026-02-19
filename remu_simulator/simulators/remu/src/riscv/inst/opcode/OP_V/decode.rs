use crate::riscv::inst::{DecodedInst, Inst, funct3, opcode::OP_V::VInst, rd, rs1, rs2};

pub(crate) const OPCODE: u32 = 0b101_0111; // OP-V
pub(crate) const INSTRUCTION_MIX: u32 = 5;

mod func3 {
    pub(super) const VSETIVLI: u32 = 0b111;
    pub(super) const OPMVV: u32 = 0b010;
    pub(super) const RSUB_VI: u32 = 0b011;
    /// vadd.vi: funct3=0, top2=00, funct6=0 (Spike MATCH_VADD_VI 0x3057, MASK 0xfc00707f)
    pub(super) const ADD_VI_F3: u32 = 0b000;
    /// vmerge.vxm: funct3=4, top2=01, funct6=0x17
    pub(super) const VMERGE_VXM: u32 = 0b100;
    /// vmv.s.x: funct3=6 (0b110), top2=01, funct6=0x10 (Spike MATCH_VMV_S_X 0x42006057, MASK 0xfff0707f)
    pub(super) const VMV_S_X: u32 = 0b110;
}

mod top2 {
    /// inst[31]=1: vsetivli (10-bit vtypei)
    pub(super) const VSETIVLI: u32 = 0b11;
    /// inst[31]=0: vsetvli (11-bit vtypei), top2 is 0 or 1
    pub(super) const VSETVLI_0: u32 = 0b00;
    pub(super) const VSETVLI_1: u32 = 0b01;

    pub(super) const VID_V: u32 = 0b01;
    pub(super) const RSUB_VI: u32 = 0b00;
    /// vmv1r.v: top2=10 (Spike MATCH_VMV1R_V 0x9e003057, MASK 0xfe0ff07f)
    pub(super) const VMV1R_V: u32 = 0b10;
}

mod funct6 {
    pub(super) const VID_V: u32 = 0x14;
    pub(super) const RSUB_VI: u32 = 0x03;
    pub(super) const ADD_VI: u32 = 0x00;
    /// vmerge.vim: merge immediate with vs2 under v0 (vm=0) or unmasked (vm=1; vmv.v.i is pseudo)
    pub(super) const VMERGE_VIM: u32 = 0x17;
    /// vmseq.vi: mask where vs2[i] == simm5
    pub(super) const VMSEQ_VI: u32 = 0x18;
    /// vsext.vf4: sign-extend vs2 from SEW/4 to SEW
    pub(super) const VSEXT_VF4: u32 = 0x12;
    /// vmv.s.x: vd[0] = rs1 (scalar), rest unchanged
    pub(super) const VMV_S_X: u32 = 0x10;
    /// vmv1r.v: copy one vector register vd = vs2
    pub(super) const VMV1R_V: u32 = 0x27;
    /// vmslt.vx: vd[i] = (vs2[i] < rs1) ? 1 : 0 (signed, mask) — Spike MATCH 0x6c004057
    pub(super) const VMSLT_VX: u32 = 0x1b;
    /// vredsum.vs: vd[0] = vs1[0] + sum(vs2[0..vl]) (Spike MATCH 0x2057, MASK 0xfc00707f)
    pub(super) const VREDSUM_VS: u32 = 0x00;
    /// vslidedown.vi: vd[i] = vs2[i+uimm5] if (i+uimm5)<vl else 0 (Spike MATCH 0x3c003057)
    pub(super) const VSLIDEDOWN_VI: u32 = 0x0f;
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);
    let top2 = top2(inst);

    let v_inst = match (f3, top2) {
        (func3::VSETIVLI, top2::VSETIVLI) => VInst::Vsetivli,
        (func3::VSETIVLI, top2::VSETVLI_0) | (func3::VSETIVLI, top2::VSETVLI_1) => VInst::Vsetvli,
        (func3::OPMVV, top2::RSUB_VI) => {
            if funct6(inst) == funct6::VREDSUM_VS {
                VInst::Vredsum_vs
            } else {
                return DecodedInst::default();
            }
        }
        (func3::OPMVV, top2::VID_V) => {
            if funct6(inst) == funct6::VID_V && vm(inst) == 1 && rs2(inst) == 0 {
                VInst::Vid_v
            } else if funct6(inst) == funct6::VMV_S_X {
                // Same encoding family: Spike MATCH_VMV_X_S has rs1=0, MATCH_VFIRST_M has rs1=20.
                // Distinguish by rs1 ([19:15]): rs1==0 -> vmv.x.s, rs1!=0 -> vfirst.m.
                if rs1(inst) == 0 {
                    VInst::Vmv_x_s
                } else {
                    VInst::Vfirst_m
                }
            } else if funct6(inst) == funct6::VMERGE_VIM {
                VInst::Vmerge_vim
            } else if funct6(inst) == funct6::VMSEQ_VI {
                VInst::Vmseq_vi
            } else if funct6(inst) == funct6::VSEXT_VF4 {
                VInst::Vsext_vf4
            } else {
                return DecodedInst::default();
            }
        }
        (func3::RSUB_VI, top2::VID_V) => {
            if funct6(inst) == funct6::VMERGE_VIM {
                VInst::Vmerge_vim
            } else if funct6(inst) == funct6::VMSEQ_VI {
                VInst::Vmseq_vi
            } else if funct6(inst) == funct6::VSEXT_VF4 {
                VInst::Vsext_vf4
            } else {
                return DecodedInst::default();
            }
        }
        (func3::VMERGE_VXM, top2::VID_V) => {
            if funct6(inst) == funct6::VMERGE_VIM {
                VInst::Vmerge_vxm
            } else if funct6(inst) == funct6::VMSLT_VX {
                VInst::Vmslt_vx
            } else {
                return DecodedInst::default();
            }
        }
        (func3::VMV_S_X, top2::VID_V) => {
            if funct6(inst) == funct6::VMV_S_X {
                VInst::Vmv_s_x
            } else {
                return DecodedInst::default();
            }
        }
        (func3::RSUB_VI, top2::VMV1R_V) => {
            if funct6(inst) == funct6::VMV1R_V {
                VInst::Vmv1r_v
            } else {
                return DecodedInst::default();
            }
        }
        (func3::ADD_VI_F3, top2::RSUB_VI) => {
            if funct6(inst) == funct6::ADD_VI {
                VInst::Vadd_vi
            } else {
                return DecodedInst::default();
            }
        }
        (func3::RSUB_VI, top2::RSUB_VI) => {
            if funct6(inst) == funct6::RSUB_VI {
                VInst::Vrsub_vi
            } else if funct6(inst) == funct6::ADD_VI {
                // vadd.vi: also (funct3=3, top2=0), funct6=0 (e.g. 0x00003057)
                VInst::Vadd_vi
            } else if funct6(inst) == 0x0a {
                // vadd.vi alternate encoding (e.g. 0x02843457, ref/difftest use this)
                VInst::Vadd_vi
            } else if funct6(inst) == funct6::VSLIDEDOWN_VI {
                VInst::Vslidedown_vi
            } else {
                return DecodedInst::default();
            }
        }
        _ => return DecodedInst::default(),
    };

    match v_inst {
        VInst::Vsetivli => DecodedInst {
            rd: rd(inst),
            rs1: 0,
            rs2: 0,
            imm: ((inst >> 15) & 0x7FFF),
            inst: Inst::V(VInst::Vsetivli),
        },
        VInst::Vsetvli => DecodedInst {
            rd: rd(inst),
            rs1: rs1(inst),
            rs2: 0,
            imm: (inst >> 15) & 0x7FFF,
            inst: Inst::V(VInst::Vsetvli),
        },
        VInst::Vid_v => DecodedInst {
            rd: rd(inst),
            rs1: 0,
            rs2: 0,
            imm: 0,
            inst: Inst::V(VInst::Vid_v),
        },
        VInst::Vrsub_vi => {
            let simm5 = ((rs1(inst) as i32) << 27 >> 27) as u32;
            DecodedInst {
                rd: rd(inst),
                rs1: 0,
                rs2: rs2(inst),
                imm: simm5,
                inst: Inst::V(VInst::Vrsub_vi),
            }
        }
        VInst::Vadd_vi => {
            let simm5 = ((rs1(inst) as i32) << 27 >> 27) as u32;
            DecodedInst {
                rd: rd(inst),
                rs1: vm(inst) as u8, // vm: 0 = use v0 mask, 1 = unmasked
                rs2: rs2(inst),
                imm: simm5,
                inst: Inst::V(VInst::Vadd_vi),
            }
        }
        VInst::Vmerge_vim => {
            let simm5 = ((rs1(inst) as i32) << 27 >> 27) as u32;
            DecodedInst {
                rd: rd(inst),
                rs1: vm(inst) as u8, // vm: 0 = use v0 mask, 1 = unmasked (all 1s)
                rs2: rs2(inst),
                imm: simm5,
                inst: Inst::V(VInst::Vmerge_vim),
            }
        }
        VInst::Vmseq_vi => {
            let simm5 = ((rs1(inst) as i32) << 27 >> 27) as u32;
            DecodedInst {
                rd: rd(inst),
                rs1: 0,
                rs2: rs2(inst),
                imm: simm5,
                inst: Inst::V(VInst::Vmseq_vi),
            }
        }
        VInst::Vmerge_vxm => DecodedInst {
            rd: rd(inst),
            rs1: rs1(inst), // GPR for scalar
            rs2: rs2(inst), // vs2
            imm: 0,
            inst: Inst::V(VInst::Vmerge_vxm),
        },
        VInst::Vsext_vf4 => DecodedInst {
            rd: rd(inst),
            rs1: 0,
            rs2: rs2(inst),
            imm: 0,
            inst: Inst::V(VInst::Vsext_vf4),
        },
        VInst::Vmv_s_x => DecodedInst {
            rd: rd(inst),
            rs1: rs1(inst),
            rs2: 0,
            imm: 0,
            inst: Inst::V(VInst::Vmv_s_x),
        },
        VInst::Vmv1r_v => DecodedInst {
            rd: rd(inst),
            rs1: 0,
            rs2: rs2(inst), // vs2
            imm: 0,
            inst: Inst::V(VInst::Vmv1r_v),
        },
        VInst::Vmslt_vx => DecodedInst {
            rd: rd(inst),
            rs1: rs1(inst), // scalar
            rs2: rs2(inst), // vs2
            imm: 0,
            inst: Inst::V(VInst::Vmslt_vx),
        },
        VInst::Vredsum_vs => DecodedInst {
            rd: rd(inst),   // vd
            rs1: rs1(inst), // vs1 (scalar in vs1[0])
            rs2: rs2(inst), // vs2
            imm: 0,
            inst: Inst::V(VInst::Vredsum_vs),
        },
        VInst::Vmv_x_s => DecodedInst {
            rd: rd(inst),
            rs1: 0,
            rs2: rs2(inst),
            imm: 0,
            inst: Inst::V(VInst::Vmv_x_s),
        },
        VInst::Vfirst_m => DecodedInst {
            rd: rd(inst),
            rs1: 0,
            rs2: rs2(inst),
            imm: 0,
            inst: Inst::V(VInst::Vfirst_m),
        },
        VInst::Vslidedown_vi => DecodedInst {
            rd: rd(inst),
            rs1: 0,
            rs2: rs2(inst),        // vs2
            imm: rs1(inst) as u32, // uimm5 (VI format: imm in rs1 field)
            inst: Inst::V(VInst::Vslidedown_vi),
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
