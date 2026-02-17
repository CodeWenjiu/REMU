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
}

mod top2 {
    pub(super) const VSETIVLI: u32 = 0b11;
    pub(super) const VID_V: u32 = 0b01;
    pub(super) const RSUB_VI: u32 = 0b00;
}

mod funct6 {
    pub(super) const VID_V: u32 = 0x14;
    pub(super) const RSUB_VI: u32 = 0x03;
    pub(super) const ADD_VI: u32 = 0x00;
    /// vmerge.vim: merge immediate with vs2 under v0 (vm=0) or unmasked (vm=1; vmv.v.i is pseudo)
    pub(super) const VMERGE_VIM: u32 = 0x17;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub(crate) enum VInst {
    Vsetivli,
    Vid_v,
    Vrsub_vi,
    Vadd_vi,
    Vmerge_vim,
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
        (func3::VID_V, top2::VID_V) => {
            if v_funct6(inst) == funct6::VID_V && v_vm(inst) == 1 && rs2(inst) == 0 {
                VInst::Vid_v
            } else if v_funct6(inst) == funct6::VMERGE_VIM {
                VInst::Vmerge_vim
            } else {
                return DecodedInst::default();
            }
        }
        (func3::RSUB_VI, top2::VID_V) => {
            if v_funct6(inst) == funct6::VMERGE_VIM {
                VInst::Vmerge_vim
            } else {
                return DecodedInst::default();
            }
        }
        (func3::RSUB_VI, top2::RSUB_VI) => match v_funct6(inst) {
            funct6::RSUB_VI => VInst::Vrsub_vi,
            funct6::ADD_VI => VInst::Vadd_vi,
            _ => return DecodedInst::default(),
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
                rs1: 0,
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
    let (num, denom) = match vlmul {
        0 => (1, 1),
        1 => (2, 1),
        2 => (4, 1),
        3 => (8, 1),
        4 => (1, 2),
        5 => (1, 4),
        6 => (1, 8),
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
/// and passes (element_index, sew_bytes, optional_rs2_value, mask_bit) to the closure.
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
    F: FnMut(u32, usize, Option<u64>, bool) -> u64,
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
        let mut dst_chunk = vec![0u8; vlenb];

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

            let res = op(i, sew_bytes, src_val, mask_bit);

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
                    |_, sew, src, mask| {
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
    }
}
