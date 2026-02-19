//! RISC-V V extension (OP-V opcode 0x57). Decode only when VLENB > 0.

use remu_types::isa::extension_v::VExtensionConfig;
use remu_types::isa::reg::{RegAccess, VectorCsrState, VrState};
use remu_types::isa::RvIsa;

use crate::riscv::inst::{DecodedInst, Inst, funct3, rd, rs1, rs2, v_funct6, v_vm};

pub(crate) const OPCODE: u32 = 0b101_0111; // OP-V
pub(crate) const INSTRUCTION_MIX: u32 = 5;

mod func3 {
    pub(super) const VSETIVLI: u32 = 0b111;
    pub(super) const VID_V: u32 = 0b010;
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
    pub(super) const VID_V: u32 = 0b01;
    pub(super) const RSUB_VI: u32 = 0b00;
    /// inst[31]=0: vsetvli (11-bit vtypei), top2 is 0 or 1
    pub(super) const VSETVLI_0: u32 = 0b00;
    pub(super) const VSETVLI_1: u32 = 0b01;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub(crate) enum VInst {
    Vsetivli,
    /// vsetvli rd, rs1, vtype: AVL from GPR rs1, 11-bit vtypei
    Vsetvli,
    Vid_v,
    Vrsub_vi,
    Vadd_vi,
    Vmerge_vim,
    /// vmseq.vi: vd[i] = (vs2[i] == simm5) ? 1 : 0 (mask, 1 bit per element)
    Vmseq_vi,
    /// vmerge.vxm: vd[i] = v0[i] ? rs1 (scalar) : vs2[i]
    Vmerge_vxm,
    /// vsext.vf4: vd[i] = sign_extend(vs2[i] from SEW/4 to SEW)
    Vsext_vf4,
    /// vmv.s.x: vd[0] = rs1 (scalar), vd[1..] unchanged
    Vmv_s_x,
    /// vmv1r.v: vd = vs2 (copy one vector register, VLEN bytes)
    Vmv1r_v,
    /// vmslt.vx: vd[i] = (vs2[i] < rs1) ? 1 : 0 (signed, mask)
    Vmslt_vx,
    /// vredsum.vs: vd[0] = vs1[0] + sum(vs2[0..vl]), scalar result in vd[0]
    Vredsum_vs,
    /// vmv.x.s: rd = sign_extend(vs2[0]) to XLEN (vm=1 only in Spike)
    Vmv_x_s,
    /// vfirst.m: rd = index of first set mask bit in vs2, or -1 if none (vm=0, same encoding family as vmv.x.s)
    Vfirst_m,
    /// vslidedown.vi: vd[i] = vs2[i+uimm5] if (i+uimm5)<vl else 0
    Vslidedown_vi,
}

#[inline(always)]
fn v_zimm_vsetivli(inst: u32) -> u32 {
    ((v_funct6(inst) & 0xF) << 6) | (v_vm(inst) << 5) | rs2(inst) as u32
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);
    let top2 = (inst >> 30) & 0x3;

    let v_inst = match (f3, top2) {
        (func3::VSETIVLI, top2::VSETIVLI) => VInst::Vsetivli,
        (func3::VSETIVLI, top2::VSETVLI_0) | (func3::VSETIVLI, top2::VSETVLI_1) => VInst::Vsetvli,
        (func3::VID_V, top2::RSUB_VI) => {
            if v_funct6(inst) == funct6::VREDSUM_VS {
                VInst::Vredsum_vs
            } else {
                return DecodedInst::default();
            }
        }
        (func3::VID_V, top2::VID_V) => {
            if v_funct6(inst) == funct6::VID_V && v_vm(inst) == 1 && rs2(inst) == 0 {
                VInst::Vid_v
            } else if v_funct6(inst) == funct6::VMV_S_X {
                // Same encoding family: Spike MATCH_VMV_X_S has rs1=0, MATCH_VFIRST_M has rs1=20.
                // Distinguish by rs1 ([19:15]): rs1==0 -> vmv.x.s, rs1!=0 -> vfirst.m.
                if rs1(inst) == 0 {
                    VInst::Vmv_x_s
                } else {
                    VInst::Vfirst_m
                }
            } else if v_funct6(inst) == funct6::VMERGE_VIM {
                VInst::Vmerge_vim
            } else if v_funct6(inst) == funct6::VMSEQ_VI {
                VInst::Vmseq_vi
            } else if v_funct6(inst) == funct6::VSEXT_VF4 {
                VInst::Vsext_vf4
            } else {
                return DecodedInst::default();
            }
        }
        (func3::RSUB_VI, top2::VID_V) => {
            if v_funct6(inst) == funct6::VMERGE_VIM {
                VInst::Vmerge_vim
            } else if v_funct6(inst) == funct6::VMSEQ_VI {
                VInst::Vmseq_vi
            } else if v_funct6(inst) == funct6::VSEXT_VF4 {
                VInst::Vsext_vf4
            } else {
                return DecodedInst::default();
            }
        }
        (func3::VMERGE_VXM, top2::VID_V) => {
            if v_funct6(inst) == funct6::VMERGE_VIM {
                VInst::Vmerge_vxm
            } else if v_funct6(inst) == funct6::VMSLT_VX {
                VInst::Vmslt_vx
            } else {
                return DecodedInst::default();
            }
        }
        (func3::VMV_S_X, top2::VID_V) => {
            if v_funct6(inst) == funct6::VMV_S_X {
                VInst::Vmv_s_x
            } else {
                return DecodedInst::default();
            }
        }
        (func3::RSUB_VI, top2::VMV1R_V) => {
            if v_funct6(inst) == funct6::VMV1R_V {
                VInst::Vmv1r_v
            } else {
                return DecodedInst::default();
            }
        }
        (func3::ADD_VI_F3, top2::RSUB_VI) => {
            if v_funct6(inst) == funct6::ADD_VI {
                VInst::Vadd_vi
            } else {
                return DecodedInst::default();
            }
        }
        (func3::RSUB_VI, top2::RSUB_VI) => {
            if v_funct6(inst) == funct6::RSUB_VI {
                VInst::Vrsub_vi
            } else if v_funct6(inst) == funct6::ADD_VI {
                // vadd.vi: also (funct3=3, top2=0), funct6=0 (e.g. 0x00003057)
                VInst::Vadd_vi
            } else if v_funct6(inst) == 0x0a {
                // vadd.vi alternate encoding (e.g. 0x02843457, ref/difftest use this)
                VInst::Vadd_vi
            } else if v_funct6(inst) == funct6::VSLIDEDOWN_VI {
                VInst::Vslidedown_vi
            } else {
                return DecodedInst::default();
            }
        },
        _ => return DecodedInst::default(),
    };

    match v_inst {
        VInst::Vsetivli => {
            let uimm = rs1(inst) as u32;
            let zimm = v_zimm_vsetivli(inst);
            DecodedInst {
                rd: rd(inst),
                rs1: 0,
                rs2: 0,
                imm: (zimm << 8) | uimm,
                inst: Inst::V(VInst::Vsetivli),
            }
        }
        VInst::Vsetvli => {
            let vtypei = (inst >> 20) & 0x7FF;
            DecodedInst {
                rd: rd(inst),
                rs1: rs1(inst),
                rs2: 0,
                imm: vtypei,
                inst: Inst::V(VInst::Vsetvli),
            }
        }
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
                rs1: v_vm(inst) as u8, // vm: 0 = use v0 mask, 1 = unmasked
                rs2: rs2(inst),
                imm: simm5,
                inst: Inst::V(VInst::Vadd_vi),
            }
        }
        VInst::Vmerge_vim => {
            let simm5 = ((rs1(inst) as i32) << 27 >> 27) as u32;
            DecodedInst {
                rd: rd(inst),
                rs1: v_vm(inst) as u8, // vm: 0 = use v0 mask, 1 = unmasked (all 1s)
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
            rs2: rs2(inst), // vs2
            imm: rs1(inst) as u32, // uimm5 (VI format: imm in rs1 field)
            inst: Inst::V(VInst::Vslidedown_vi),
        },
    }
}

#[inline(always)]
fn zimm_to_vtype(zimm: u32) -> u32 {
    (zimm & 0xFF) & !(1 << 31)
}

#[inline(always)]
fn nf_from_vlmul(vlmul: u32) -> usize {
    match vlmul & 0x7 {
        0 => 1,
        1 => 2,
        2 => 4,
        3 => 8,
        _ => 1,
    }
}

#[inline(always)]
fn vlmax_vlenb_vtype(vlenb: u32, vtype: u32) -> u32 {
    let vlen = vlenb * 8;
    let vsew = (vtype >> 3) & 0x7;
    let vlmul = vtype & 0x7;
    let sew = match vsew {
        0 => 8,
        1 => 16,
        2 => 32,
        3 => 64,
        _ => 8,
    };
    // Fractional LMUL: vlmul[2:0] is interpreted as signed (spec/Spike); 4→-4, 5→-3, 6→-2, 7→-1,
    // so LMUL = 2^vlmul → 4→1/16, 5→1/8 (mf8), 6→1/4 (mf4), 7→1/2 (mf2).
    let (num, denom) = match vlmul & 0x7 {
        0 => (1, 1),
        1 => (2, 1),
        2 => (4, 1),
        3 => (8, 1),
        4 => (1, 16), // -4 → 1/16
        5 => (1, 8),  // -3 → mf8
        6 => (1, 4),  // -2 → mf4
        7 => (1, 2),  // -1 → mf2
        _ => (1, 1),
    };
    ((vlen / sew) * num) / denom
}

/// Generic vector element loop helper.
/// Handles: V configuration (vl, vtype, sew), multi-register grouping (nf), loop bounds,
/// reading from rs2 (optional), and writing to rd.
/// `op` closure receives: (element_index, sew_bytes, optional_rs2_value).
/// `op` must return the result as u64 (bits), which will be truncated to SEW.
#[inline(always)]
fn vector_element_loop<P, C, F>(
    ctx: &mut C,
    rd: usize,
    rs2: Option<usize>,
    mut op: F,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
    F: FnMut(u32, usize, Option<u64>) -> u64,
{
    let state = ctx.state_mut();
    let vl = state.reg.csr.vector.vl();
    let vtype = state.reg.csr.vector.vtype();
    let vlmul = vtype & 0x7;
    let vsew = (vtype >> 3) & 0x7;
    let sew_bytes = 1 << (vsew & 0x3); // 0->1, 1->2, 2->4, 3->8. Ignore reserved 4-7.

    let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
    let nf_max = nf_from_vlmul(vlmul);

    // Clamp nf to avoid register file overflow (spec: illegal if > 31)
    let mut nf = nf_max.min(32_usize.saturating_sub(rd));
    if let Some(r2) = rs2 {
        nf = nf.min(32_usize.saturating_sub(r2));
    }

    let total_elems = (nf * vlenb) / sew_bytes;
    let n = vl.min(total_elems as u32);

    for r in 0..nf {
        // Read source register chunk if needed (copy to avoid borrow conflict)
        let src_chunk = rs2.map(|reg| state.reg.vr.raw_read(reg + r).to_vec());
        let mut dst_chunk = vec![0u8; vlenb];

        // Process elements belonging to this register
        // Element i is in register r if (i * sew) / vlenb == r
        let start_elem = (r * vlenb) / sew_bytes;
        let end_elem = ((r + 1) * vlenb) / sew_bytes;
        // Intersect with [0, n)
        let loop_start = (start_elem as u32).min(n);
        let loop_end = (end_elem as u32).min(n);

        for i in loop_start..loop_end {
            let off = ((i as usize) * sew_bytes) % vlenb;
            
            // Extract source operand
            let src_val = src_chunk.as_ref().map(|chunk| {
                match sew_bytes {
                    1 => chunk[off] as u64,
                    2 => u16::from_le_bytes(chunk[off..off+2].try_into().unwrap()) as u64,
                    4 => u32::from_le_bytes(chunk[off..off+4].try_into().unwrap()) as u64,
                    8 => u64::from_le_bytes(chunk[off..off+8].try_into().unwrap()),
                    _ => 0,
                }
            });

            let res = op(i, sew_bytes, src_val);

            // Write result
            match sew_bytes {
                1 => dst_chunk[off] = res as u8,
                2 => dst_chunk[off..off+2].copy_from_slice(&(res as u16).to_le_bytes()),
                4 => dst_chunk[off..off+4].copy_from_slice(&(res as u32).to_le_bytes()),
                8 => dst_chunk[off..off+8].copy_from_slice(&res.to_le_bytes()),
                _ => {}
            }
        }
        
        state.reg.vr.raw_write(rd + r, &dst_chunk);
    }

    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// Like `vector_element_loop` but reads mask from v0 (1 bit per element, LSB-packed)
/// and passes (element_index, sew_bytes, optional_rs2_value, mask_bit, dst_current) to the closure.
/// When mask_bit is false, op should return dst_current to leave vd[i] unchanged.
#[inline(always)]
fn vector_element_loop_masked<P, C, F>(
    ctx: &mut C,
    rd: usize,
    rs2: Option<usize>,
    mut op: F,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
    F: FnMut(u32, usize, Option<u64>, bool, u64) -> u64,
{
    let state = ctx.state_mut();
    let vl = state.reg.csr.vector.vl();
    let vtype = state.reg.csr.vector.vtype();
    let vlmul = vtype & 0x7;
    let vsew = (vtype >> 3) & 0x7;
    let sew_bytes = 1 << (vsew & 0x3);

    let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
    let nf_max = nf_from_vlmul(vlmul);
    let mut nf = nf_max.min(32_usize.saturating_sub(rd));
    if let Some(r2) = rs2 {
        nf = nf.min(32_usize.saturating_sub(r2));
    }

    let total_elems = (nf * vlenb) / sew_bytes;
    let n = vl.min(total_elems as u32);

    let v0 = state.reg.vr.raw_read(0).to_vec();

    for r in 0..nf {
        let src_chunk = rs2.map(|reg| state.reg.vr.raw_read(reg + r).to_vec());
        let mut dst_chunk = state.reg.vr.raw_read(rd + r).to_vec();

        let start_elem = (r * vlenb) / sew_bytes;
        let end_elem = ((r + 1) * vlenb) / sew_bytes;
        let loop_start = (start_elem as u32).min(n);
        let loop_end = (end_elem as u32).min(n);

        for i in loop_start..loop_end {
            let off = ((i as usize) * sew_bytes) % vlenb;
            let mask_bit = (v0[(i as usize) / 8] >> (i % 8)) & 1 != 0;

            let src_val = src_chunk.as_ref().map(|chunk| {
                match sew_bytes {
                    1 => chunk[off] as u64,
                    2 => u16::from_le_bytes(chunk[off..off + 2].try_into().unwrap()) as u64,
                    4 => u32::from_le_bytes(chunk[off..off + 4].try_into().unwrap()) as u64,
                    8 => u64::from_le_bytes(chunk[off..off + 8].try_into().unwrap()),
                    _ => 0,
                }
            });
            let dst_val = match sew_bytes {
                1 => dst_chunk[off] as u64,
                2 => u16::from_le_bytes(dst_chunk[off..off + 2].try_into().unwrap()) as u64,
                4 => u32::from_le_bytes(dst_chunk[off..off + 4].try_into().unwrap()) as u64,
                8 => u64::from_le_bytes(dst_chunk[off..off + 8].try_into().unwrap()),
                _ => 0,
            };

            let res = op(i, sew_bytes, src_val, mask_bit, dst_val);

            match sew_bytes {
                1 => dst_chunk[off] = res as u8,
                2 => dst_chunk[off..off + 2].copy_from_slice(&(res as u16).to_le_bytes()),
                4 => dst_chunk[off..off + 4].copy_from_slice(&(res as u32).to_le_bytes()),
                8 => dst_chunk[off..off + 8].copy_from_slice(&res.to_le_bytes()),
                _ => {}
            }
        }

        state.reg.vr.raw_write(rd + r, &dst_chunk);
    }

    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// vsext.vf4: vd[i] = sign_extend(vs2[i] from SEW/4 to SEW). EEW = SEW/4.
#[inline(always)]
fn vector_sext_vf4<P, C>(
    ctx: &mut C,
    vd: usize,
    vs2: usize,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let state = ctx.state_mut();
    let vl = state.reg.csr.vector.vl();
    let vtype = state.reg.csr.vector.vtype();
    let vlmul = vtype & 0x7;
    let vsew = (vtype >> 3) & 0x7;
    let sew_bytes = 1 << (vsew & 0x3);
    let src_sew_bytes = sew_bytes / 4;
    if src_sew_bytes == 0 {
        return Err(remu_state::StateError::BusError(Box::new(
            remu_state::bus::BusError::unmapped(0),
        )));
    }
    let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
    let nf = nf_from_vlmul(vlmul)
        .min(32_usize.saturating_sub(vd))
        .min(32_usize.saturating_sub(vs2));
    let total_elems = (nf * vlenb) / sew_bytes;
    let n = vl.min(total_elems as u32) as usize;

    for r in 0..nf {
        let mut dst_chunk = state.reg.vr.raw_read(vd + r).to_vec();
        let start_elem = (r * vlenb) / sew_bytes;
        let end_elem = ((r + 1) * vlenb) / sew_bytes;
        let loop_end = end_elem.min(n);

        for i in start_elem..loop_end {
            let src_byte_off = i * src_sew_bytes;
            let src_reg = src_byte_off / vlenb;
            let src_off = src_byte_off % vlenb;
            let src_chunk = state.reg.vr.raw_read(vs2 + src_reg);
            let mut val = [0u8; 8];
            match (src_sew_bytes, sew_bytes) {
                (1, 4) => {
                    let b = src_chunk[src_off] as i8;
                    val[..4].copy_from_slice(&(b as i32 as u32).to_le_bytes());
                }
                (1, 8) => {
                    let b = src_chunk[src_off] as i8;
                    val[..8].copy_from_slice(&(b as i64 as u64).to_le_bytes());
                }
                (2, 4) => {
                    let w = i16::from_le_bytes(src_chunk[src_off..src_off + 2].try_into().unwrap());
                    val[..4].copy_from_slice(&(w as i32 as u32).to_le_bytes());
                }
                (2, 8) => {
                    let w = i16::from_le_bytes(src_chunk[src_off..src_off + 2].try_into().unwrap());
                    val[..8].copy_from_slice(&(w as i64 as u64).to_le_bytes());
                }
                _ => continue,
            }
            let dst_off = (i * sew_bytes) % vlenb;
            dst_chunk[dst_off..dst_off + sew_bytes].copy_from_slice(&val[..sew_bytes]);
        }
        state.reg.vr.raw_write(vd + r, &dst_chunk);
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// vmseq.vi: write mask to vd (1 bit per element). vd[i] = (vs2[i] == simm5_sext) ? 1 : 0.
/// vd is a single register; tail bits (vl..VLEN) are left unchanged.
#[inline(always)]
fn vector_mask_cmp_vi<P, C>(
    ctx: &mut C,
    vd: usize,
    vs2: usize,
    simm5: u32,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let state = ctx.state_mut();
    let vl = state.reg.csr.vector.vl();
    let vtype = state.reg.csr.vector.vtype();
    let vlmul = vtype & 0x7;
    let vsew = (vtype >> 3) & 0x7;
    let sew_bytes = 1 << (vsew & 0x3);
    let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
    let nf = nf_from_vlmul(vlmul).min(32_usize.saturating_sub(vs2));
    let total_elems = (nf * vlenb) / sew_bytes;
    let n = vl.min(total_elems as u32) as usize;

    let mut vd_buf = state.reg.vr.raw_read(vd).to_vec();

    for i in 0..n {
        let byte_offset = (i as usize) * sew_bytes;
        let reg_i = byte_offset / vlenb;
        let off = byte_offset % vlenb;
        let chunk = state.reg.vr.raw_read(vs2 + reg_i);
        let vs2_val = match sew_bytes {
            1 => chunk[off] as u64,
            2 => u16::from_le_bytes(chunk[off..off + 2].try_into().unwrap()) as u64,
            4 => u32::from_le_bytes(chunk[off..off + 4].try_into().unwrap()) as u64,
            8 => u64::from_le_bytes(chunk[off..off + 8].try_into().unwrap()),
            _ => 0,
        };
        let imm_sext = match sew_bytes {
            1 => (simm5 as i8) as u8 as u64,
            2 => (simm5 as i16) as u16 as u64,
            4 => (simm5 as i32) as u32 as u64,
            8 => (simm5 as i64) as u64,
            _ => 0,
        };
        let bit = (vs2_val == imm_sext) as u8;
        let byte_idx = i / 8;
        let bit_idx = i % 8;
        if bit != 0 {
            vd_buf[byte_idx] |= 1u8 << bit_idx;
        } else {
            vd_buf[byte_idx] &= !(1u8 << bit_idx);
        }
    }

    state.reg.vr.raw_write(vd, &vd_buf);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// vredsum.vs: vd[0] = vs1[0] + sum(vs2[0..vl]) (signed, SEW).
#[inline(always)]
fn vector_redsum_vs<P, C>(
    ctx: &mut C,
    vd: usize,
    vs1: usize,
    vs2: usize,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let state = ctx.state_mut();
    let vl = state.reg.csr.vector.vl();
    let vtype = state.reg.csr.vector.vtype();
    let vlmul = vtype & 0x7;
    let vsew = (vtype >> 3) & 0x7;
    let sew_bytes = 1 << (vsew & 0x3);
    let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
    let nf = nf_from_vlmul(vlmul).min(32_usize.saturating_sub(vs2));
    let total_elems = (nf * vlenb) / sew_bytes;
    let n = vl.min(total_elems as u32) as usize;

    let vs1_chunk = state.reg.vr.raw_read(vs1);
    let acc_sew = match sew_bytes {
        1 => vs1_chunk[0] as i8 as i64,
        2 => i16::from_le_bytes(vs1_chunk[0..2].try_into().unwrap()) as i64,
        4 => i32::from_le_bytes(vs1_chunk[0..4].try_into().unwrap()) as i64,
        8 => i64::from_le_bytes(vs1_chunk[0..8].try_into().unwrap()),
        _ => 0,
    };
    let mut acc = acc_sew;

    for i in 0..n {
        let byte_offset = i * sew_bytes;
        let reg_i = byte_offset / vlenb;
        let off = byte_offset % vlenb;
        let chunk = state.reg.vr.raw_read(vs2 + reg_i);
        let vs2_val = match sew_bytes {
            1 => chunk[off] as i8 as i64,
            2 => i16::from_le_bytes(chunk[off..off + 2].try_into().unwrap()) as i64,
            4 => i32::from_le_bytes(chunk[off..off + 4].try_into().unwrap()) as i64,
            8 => i64::from_le_bytes(chunk[off..off + 8].try_into().unwrap()),
            _ => 0,
        };
        acc = acc.wrapping_add(vs2_val);
    }

    let mut vd_chunk = state.reg.vr.raw_read(vd).to_vec();
    match sew_bytes {
        1 => vd_chunk[0] = (acc as i8) as u8,
        2 => vd_chunk[0..2].copy_from_slice(&(acc as i16).to_le_bytes()),
        4 => vd_chunk[0..4].copy_from_slice(&(acc as i32).to_le_bytes()),
        8 => vd_chunk[0..8].copy_from_slice(&acc.to_le_bytes()),
        _ => {}
    }
    state.reg.vr.raw_write(vd, &vd_chunk);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// vslidedown.vi: vd[i] = vs2[i+sh] if (i+sh)<vl else 0; tail zeroed.
#[inline(always)]
fn vector_slidedown_vi<P, C>(
    ctx: &mut C,
    vd: usize,
    vs2: usize,
    uimm5: u32,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let state = ctx.state_mut();
    let vl = state.reg.csr.vector.vl();
    let vtype = state.reg.csr.vector.vtype();
    let vlmul = vtype & 0x7;
    let vsew = (vtype >> 3) & 0x7;
    let sew_bytes = 1 << (vsew & 0x3);
    let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
    let nf = nf_from_vlmul(vlmul)
        .min(32_usize.saturating_sub(vd))
        .min(32_usize.saturating_sub(vs2));
    let total_elems = (nf * vlenb) / sew_bytes;
    let n = vl.min(total_elems as u32) as usize;
    let sh = uimm5 as usize;

    for r in 0..nf {
        let mut dst_chunk = vec![0u8; vlenb];
        let start_elem = (r * vlenb) / sew_bytes;
        let end_elem = ((r + 1) * vlenb) / sew_bytes;
        let loop_end = end_elem.min(n);

        for i in start_elem..loop_end {
            let src_idx = i.wrapping_add(sh);
            let val = if src_idx < vl as usize {
                let byte_off = src_idx * sew_bytes;
                let reg_i = byte_off / vlenb;
                let off = byte_off % vlenb;
                let chunk = state.reg.vr.raw_read(vs2 + reg_i);
                match sew_bytes {
                    1 => chunk[off] as u64,
                    2 => u16::from_le_bytes(chunk[off..off + 2].try_into().unwrap()) as u64,
                    4 => u32::from_le_bytes(chunk[off..off + 4].try_into().unwrap()) as u64,
                    8 => u64::from_le_bytes(chunk[off..off + 8].try_into().unwrap()),
                    _ => 0,
                }
            } else {
                0
            };
            let dst_off = (i * sew_bytes) % vlenb;
            match sew_bytes {
                1 => dst_chunk[dst_off] = val as u8,
                2 => dst_chunk[dst_off..dst_off + 2].copy_from_slice(&(val as u16).to_le_bytes()),
                4 => dst_chunk[dst_off..dst_off + 4].copy_from_slice(&(val as u32).to_le_bytes()),
                8 => dst_chunk[dst_off..dst_off + 8].copy_from_slice(&val.to_le_bytes()),
                _ => {}
            }
        }
        state.reg.vr.raw_write(vd + r, &dst_chunk);
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

/// vmslt.vx: write mask to vd. vd[i] = (vs2[i] < rs1_sext) ? 1 : 0 (signed).
#[inline(always)]
fn vector_mask_cmp_vx<P, C>(
    ctx: &mut C,
    vd: usize,
    vs2: usize,
    rs1: u8,
) -> Result<(), remu_state::StateError>
where
    P: remu_state::StatePolicy,
    C: crate::ExecuteContext<P>,
{
    let state = ctx.state_mut();
    let vl = state.reg.csr.vector.vl();
    let vtype = state.reg.csr.vector.vtype();
    let vlmul = vtype & 0x7;
    let vsew = (vtype >> 3) & 0x7;
    let sew_bytes = 1 << (vsew & 0x3);
    let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
    let nf = nf_from_vlmul(vlmul).min(32_usize.saturating_sub(vs2));
    let total_elems = (nf * vlenb) / sew_bytes;
    let n = vl.min(total_elems as u32) as usize;
    let scalar = state.reg.gpr.raw_read(rs1.into());

    let mut vd_buf = state.reg.vr.raw_read(vd).to_vec();

    for i in 0..n {
        let byte_offset = (i as usize) * sew_bytes;
        let reg_i = byte_offset / vlenb;
        let off = byte_offset % vlenb;
        let chunk = state.reg.vr.raw_read(vs2 + reg_i);
        let vs2_val = match sew_bytes {
            1 => chunk[off] as i8 as i64,
            2 => i16::from_le_bytes(chunk[off..off + 2].try_into().unwrap()) as i64,
            4 => i32::from_le_bytes(chunk[off..off + 4].try_into().unwrap()) as i64,
            8 => i64::from_le_bytes(chunk[off..off + 8].try_into().unwrap()),
            _ => 0,
        };
        let rs1_sext = match sew_bytes {
            1 => (scalar as i8) as i64,
            2 => (scalar as i16) as i64,
            4 => (scalar as i32) as i64,
            8 => scalar as i64,
            _ => 0,
        };
        let bit = (vs2_val < rs1_sext) as u8;
        let byte_idx = i / 8;
        let bit_idx = i % 8;
        if bit != 0 {
            vd_buf[byte_idx] |= 1u8 << bit_idx;
        } else {
            vd_buf[byte_idx] &= !(1u8 << bit_idx);
        }
    }

    state.reg.vr.raw_write(vd, &vd_buf);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    let v = match decoded.inst {
        Inst::V(v) => v,
        _ => return crate::riscv::inst::opcode::UNKNOWN::execute::<P, C>(ctx, decoded),
    };

    match v {
        VInst::Vsetivli => {
            let zimm = (decoded.imm >> 8) & 0x3FF;
            let uimm = decoded.imm & 0x1F;
            let rd = decoded.rd;
            let vtype = zimm_to_vtype(zimm);
            let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB;
            let vlmax = vlmax_vlenb_vtype(vlenb, vtype);
            let vl = uimm.min(vlmax);

            let state = ctx.state_mut();
            state.reg.csr.vector.set_vtype(vtype);
            state.reg.csr.vector.set_vl(vl);
            state.reg.gpr.raw_write(rd.into(), vl);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
        VInst::Vsetvli => {
            let vtypei = decoded.imm & 0x7FF;
            let vtype = zimm_to_vtype(vtypei);
            let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB;
            let vlmax = vlmax_vlenb_vtype(vlenb, vtype);
            let rd = decoded.rd;
            let rs1 = decoded.rs1;

            let state = ctx.state_mut();
            state.reg.csr.vector.set_vtype(vtype);

            let vl = if vlmax == 0 {
                0
            } else if rd == 0 && rs1 == 0 {
                // rs1=x0, rd=x0: retain current VL (spec/Spike)
                state.reg.csr.vector.vl()
            } else if rd != 0 && rs1 == 0 {
                // rs1=x0, rd!=x0: set vl = vlmax
                vlmax
            } else {
                let avl = state.reg.gpr.raw_read(rs1.into());
                if avl == 0xFFFF_FFFF {
                    vlmax
                } else {
                    (avl as u32).min(vlmax)
                }
            };

            state.reg.csr.vector.set_vl(vl);
            if rd != 0 {
                state.reg.gpr.raw_write(rd.into(), vl);
            }
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
        VInst::Vid_v => {
            vector_element_loop(ctx, decoded.rd as usize, None, |idx, _, _| idx as u64)
        }
        VInst::Vrsub_vi => {
            let simm5 = decoded.imm as i32;
            vector_element_loop(ctx, decoded.rd as usize, Some(decoded.rs2 as usize), |_, sew, src| {
                let v = src.unwrap_or(0);
                match sew {
                    1 => (simm5 as i8).wrapping_sub(v as i8) as u8 as u64,
                    2 => (simm5 as i16).wrapping_sub(v as i16) as u16 as u64,
                    4 => simm5.wrapping_sub(v as i32) as u32 as u64,
                    8 => (simm5 as i64).wrapping_sub(v as i64) as u64,
                    _ => 0,
                }
            })
        }
        VInst::Vadd_vi => {
            let simm5 = decoded.imm as i32;
            let vm = decoded.rs1 != 0;
            if vm {
                vector_element_loop(ctx, decoded.rd as usize, Some(decoded.rs2 as usize), |_, sew, src| {
                    let v = src.unwrap_or(0);
                    match sew {
                        1 => (simm5 as i8).wrapping_add(v as i8) as u8 as u64,
                        2 => (simm5 as i16).wrapping_add(v as i16) as u16 as u64,
                        4 => simm5.wrapping_add(v as i32) as u32 as u64,
                        8 => (simm5 as i64).wrapping_add(v as i64) as u64,
                        _ => 0,
                    }
                })
            } else {
                vector_element_loop_masked(
                    ctx,
                    decoded.rd as usize,
                    Some(decoded.rs2 as usize),
                    |_, sew, src, mask, dst| {
                        if mask {
                            let v = src.unwrap_or(0);
                            match sew {
                                1 => (simm5 as i8).wrapping_add(v as i8) as u8 as u64,
                                2 => (simm5 as i16).wrapping_add(v as i16) as u16 as u64,
                                4 => simm5.wrapping_add(v as i32) as u32 as u64,
                                8 => (simm5 as i64).wrapping_add(v as i64) as u64,
                                _ => 0,
                            }
                        } else {
                            dst
                        }
                    },
                )
            }
        }
        VInst::Vmerge_vim => {
            let simm5 = decoded.imm as i32;
            let vm = decoded.rs1 != 0; // 1 = unmasked (vmv.v.i pseudo), 0 = use v0
            if vm {
                // Unmasked: vd[i] = imm for all i (real instruction; vmv.v.i is pseudo for this)
                vector_element_loop(ctx, decoded.rd as usize, None, |_, sew, _| {
                    match sew {
                        1 => (simm5 as i8) as u8 as u64,
                        2 => (simm5 as i16) as u16 as u64,
                        4 => (simm5 as u32) as u64,
                        8 => simm5 as i64 as u64,
                        _ => 0,
                    }
                })
            } else {
                // Masked: vd[i] = v0[i] ? imm : vs2[i]
                vector_element_loop_masked(
                    ctx,
                    decoded.rd as usize,
                    Some(decoded.rs2 as usize),
                    |_, sew, src, mask, _dst| {
                        if mask {
                            match sew {
                                1 => (simm5 as i8) as u8 as u64,
                                2 => (simm5 as i16) as u16 as u64,
                                4 => (simm5 as u32) as u64,
                                8 => simm5 as i64 as u64,
                                _ => 0,
                            }
                        } else {
                            src.unwrap_or(0)
                        }
                    },
                )
            }
        }
        VInst::Vmseq_vi => vector_mask_cmp_vi::<P, C>(
            ctx,
            decoded.rd as usize,
            decoded.rs2 as usize,
            decoded.imm,
        ),
        VInst::Vmslt_vx => vector_mask_cmp_vx::<P, C>(
            ctx,
            decoded.rd as usize,
            decoded.rs2 as usize,
            decoded.rs1,
        ),
        VInst::Vredsum_vs => vector_redsum_vs::<P, C>(
            ctx,
            decoded.rd as usize,
            decoded.rs1 as usize,
            decoded.rs2 as usize,
        ),
        VInst::Vslidedown_vi => vector_slidedown_vi::<P, C>(
            ctx,
            decoded.rd as usize,
            decoded.rs2 as usize,
            decoded.imm,
        ),
        VInst::Vmv_x_s => {
            let state = ctx.state_mut();
            let vtype = state.reg.csr.vector.vtype();
            let vsew = (vtype >> 3) & 0x7;
            let sew_bytes = 1 << (vsew & 0x3);
            let vs2_chunk = state.reg.vr.raw_read(decoded.rs2 as usize);
            let res = match sew_bytes {
                1 => (vs2_chunk[0] as i8 as i64) as u64,
                2 => (i16::from_le_bytes(vs2_chunk[0..2].try_into().unwrap()) as i64) as u64,
                4 => (i32::from_le_bytes(vs2_chunk[0..4].try_into().unwrap()) as i64) as u64,
                8 => i64::from_le_bytes(vs2_chunk[0..8].try_into().unwrap()) as u64,
                _ => 0,
            };
            state.reg.gpr.raw_write(decoded.rd.into(), res as u32);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
        VInst::Vfirst_m => {
            let state = ctx.state_mut();
            let vl = state.reg.csr.vector.vl();
            let vs2_chunk = state.reg.vr.raw_read(decoded.rs2 as usize);
            let mut pos = !0u32; // -1
            for i in 0..vl {
                let byte_idx = (i as usize) / 8;
                let bit_idx = i % 8;
                if byte_idx < vs2_chunk.len() && (vs2_chunk[byte_idx] >> bit_idx) & 1 != 0 {
                    pos = i;
                    break;
                }
            }
            state.reg.gpr.raw_write(decoded.rd.into(), pos);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
        VInst::Vsext_vf4 => vector_sext_vf4::<P, C>(
            ctx,
            decoded.rd as usize,
            decoded.rs2 as usize,
        ),
        VInst::Vmv_s_x => {
            let state = ctx.state_mut();
            let vl = state.reg.csr.vector.vl();
            if vl == 0 {
                *state.reg.pc = state.reg.pc.wrapping_add(4);
                return Ok(());
            }
            let vtype = state.reg.csr.vector.vtype();
            let vsew = (vtype >> 3) & 0x7;
            let sew_bytes = 1 << (vsew & 0x3);
            let scalar = state.reg.gpr.raw_read(decoded.rs1.into());
            let mut chunk = state.reg.vr.raw_read(decoded.rd as usize).to_vec();
            match sew_bytes {
                1 => chunk[0] = (scalar & 0xFF) as u8,
                2 => chunk[0..2].copy_from_slice(&(scalar & 0xFFFF).to_le_bytes()),
                4 => chunk[0..4].copy_from_slice(&(scalar & 0xFFFF_FFFF).to_le_bytes()),
                8 => chunk[0..8].copy_from_slice(&(scalar as u64).to_le_bytes()),
                _ => {}
            }
            state.reg.vr.raw_write(decoded.rd as usize, &chunk);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
        VInst::Vmerge_vxm => {
            let scalar = ctx.state_mut().reg.gpr.raw_read(decoded.rs1.into());
            vector_element_loop_masked(
                ctx,
                decoded.rd as usize,
                Some(decoded.rs2 as usize),
                |_, sew, src, mask, _dst| {
                    if mask {
                        match sew {
                            1 => (scalar as i32 as i8) as u8 as u64,
                            2 => (scalar as i32 as i16) as u16 as u64,
                            4 => (scalar as i32 as u32) as u64,
                            8 => (scalar as i32 as i64) as u64,
                            _ => 0,
                        }
                    } else {
                        src.unwrap_or(0)
                    }
                },
            )
        }
        VInst::Vmv1r_v => {
            let state = ctx.state_mut();
            let vs2 = decoded.rs2 as usize;
            let vd = decoded.rd as usize;
            let data = state.reg.vr.raw_read(vs2).to_vec();
            state.reg.vr.raw_write(vd, &data);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
    }
}
