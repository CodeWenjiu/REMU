//! Generic vector loop primitives. Each primitive captures a common execution pattern.
//! Instruction handlers compose these primitives with minimal per-instruction logic.

use super::utils::VectorElementLoopMode;

/// Helper: mode from vm bit (imm bit 8).
#[inline]
pub(crate) fn mode_from_vm(vm: bool) -> VectorElementLoopMode {
    if vm {
        VectorElementLoopMode::Unmasked
    } else {
        VectorElementLoopMode::Masked
    }
}

/// SEW-width binops: (scalar, src) -> result. Used by vadd_vi, vrsub_vi, vand_vi, vsll_vi, vsrl_vi.
#[inline]
pub(crate) fn binop_add_vi(simm5: i32, src: u64, sew: usize) -> u64 {
    match sew {
        1 => (simm5 as i8).wrapping_add(src as i8) as u8 as u64,
        2 => (simm5 as i16).wrapping_add(src as i16) as u16 as u64,
        4 => simm5.wrapping_add(src as i32) as u32 as u64,
        8 => (simm5 as i64).wrapping_add(src as i64) as u64,
        _ => 0,
    }
}

#[inline]
pub(crate) fn binop_sub_vi(simm5: i32, src: u64, sew: usize) -> u64 {
    match sew {
        1 => (simm5 as i8).wrapping_sub(src as i8) as u8 as u64,
        2 => (simm5 as i16).wrapping_sub(src as i16) as u16 as u64,
        4 => simm5.wrapping_sub(src as i32) as u32 as u64,
        8 => (simm5 as i64).wrapping_sub(src as i64) as u64,
        _ => 0,
    }
}

#[inline]
pub(crate) fn binop_and_vi(simm5: i32, src: u64, sew: usize) -> u64 {
    match sew {
        1 => (src as u8 & (simm5 as i8 as u8)) as u64,
        2 => (src as u16 & (simm5 as i16 as u16)) as u64,
        4 => (src as u32 & simm5 as u32) as u64,
        8 => src & (simm5 as i64 as u64),
        _ => 0,
    }
}

#[inline]
pub(crate) fn binop_shl_vi(uimm5: u32, src: u64, sew: usize) -> u64 {
    let bw = (sew * 8) as u32;
    let shamt = uimm5 & (bw - 1);
    match sew {
        1 => (src as u8).wrapping_shl(shamt) as u64,
        2 => (src as u16).wrapping_shl(shamt) as u64,
        4 => (src as u32).wrapping_shl(shamt) as u64,
        8 => (src as u64).wrapping_shl(shamt),
        _ => 0,
    }
}

#[inline]
pub(crate) fn binop_shr_vi(uimm5: u32, src: u64, sew: usize) -> u64 {
    let bw = (sew * 8) as u32;
    let shamt = uimm5 & (bw - 1);
    match sew {
        1 => (src as u8).wrapping_shr(shamt) as u64,
        2 => (src as u16).wrapping_shr(shamt) as u64,
        4 => (src as u32).wrapping_shr(shamt) as u64,
        8 => (src as u64).wrapping_shr(shamt),
        _ => 0,
    }
}

/// `vsll.vx`: shift amount = `rs1 & (SEW-1)` (Spike `VI_VX_ULOOP`).
#[inline]
pub(crate) fn binop_shl_vx(rs1: u32, src: u64, sew: usize) -> u64 {
    let bw = (sew * 8) as u32;
    let shamt = rs1 & (bw - 1);
    match sew {
        1 => (src as u8).wrapping_shl(shamt) as u64,
        2 => (src as u16).wrapping_shl(shamt) as u64,
        4 => (src as u32).wrapping_shl(shamt) as u64,
        8 => (src as u64).wrapping_shl(shamt),
        _ => 0,
    }
}

/// `vsrl.vx`: logical right shift; shamt = `rs1 & (SEW-1)`.
#[inline]
pub(crate) fn binop_shr_vx(rs1: u32, src: u64, sew: usize) -> u64 {
    let bw = (sew * 8) as u32;
    let shamt = rs1 & (bw - 1);
    match sew {
        1 => (src as u8).wrapping_shr(shamt) as u64,
        2 => (src as u16).wrapping_shr(shamt) as u64,
        4 => (src as u32).wrapping_shr(shamt) as u64,
        8 => (src as u64).wrapping_shr(shamt),
        _ => 0,
    }
}

/// vv: vd = vs2 - vs1 (signed wrap at SEW). `src1`=vs1, `src2`=vs2.
#[inline]
pub(crate) fn binop_sub_vv(src1: u64, src2: u64, sew: usize) -> u64 {
    match sew {
        1 => ((src2 as i8).wrapping_sub(src1 as i8)) as u8 as u64,
        2 => ((src2 as i16).wrapping_sub(src1 as i16)) as u16 as u64,
        4 => ((src2 as i32).wrapping_sub(src1 as i32)) as u32 as u64,
        8 => ((src2 as i64).wrapping_sub(src1 as i64)) as u64,
        _ => 0,
    }
}

/// vmax.vv: signed max(vs1, vs2). `src1`=vs1, `src2`=vs2 (same as Spike VI_VV_LOOP).
#[inline]
pub(crate) fn binop_max_vv(src1: u64, src2: u64, sew: usize) -> u64 {
    match sew {
        1 => {
            let a = src1 as u8 as i8;
            let b = src2 as u8 as i8;
            if a >= b {
                src1 & 0xff
            } else {
                src2 & 0xff
            }
        }
        2 => {
            let a = src1 as u16 as i16;
            let b = src2 as u16 as i16;
            if a >= b {
                src1 & 0xffff
            } else {
                src2 & 0xffff
            }
        }
        4 => {
            let a = src1 as u32 as i32;
            let b = src2 as u32 as i32;
            if a >= b {
                src1 & 0xffff_ffff
            } else {
                src2 & 0xffff_ffff
            }
        }
        8 => {
            let a = src1 as i64;
            let b = src2 as i64;
            if a >= b {
                src1
            } else {
                src2
            }
        }
        _ => 0,
    }
}

/// Vx binops (scalar from GPR).
#[inline]
pub(crate) fn binop_add_vx(scalar: u64, src: u64, sew: usize) -> u64 {
    match sew {
        1 => (src as u8).wrapping_add(scalar as u8) as u64,
        2 => (src as u16).wrapping_add(scalar as u16) as u64,
        4 => (src as u32).wrapping_add(scalar as u32) as u64,
        8 => (src as u64).wrapping_add(scalar),
        _ => 0,
    }
}

#[inline]
pub(crate) fn binop_and_vx(scalar: u64, src: u64, sew: usize) -> u64 {
    match sew {
        1 => (src as u8 & scalar as u8) as u64,
        2 => (src as u16 & scalar as u16) as u64,
        4 => (src as u32 & scalar as u32) as u64,
        8 => src & scalar,
        _ => 0,
    }
}

/// Merge: when mask, write scalar (truncated to SEW); else keep src.
#[inline]
pub(crate) fn merge_scalar_vi(simm5: i32, sew: usize) -> u64 {
    match sew {
        1 => (simm5 as i8) as u8 as u64,
        2 => (simm5 as i16) as u16 as u64,
        4 => (simm5 as u32) as u64,
        8 => simm5 as i64 as u64,
        _ => 0,
    }
}

#[inline]
pub(crate) fn merge_scalar_vx(scalar: u32, sew: usize) -> u64 {
    match sew {
        1 => (scalar as i8) as u8 as u64,
        2 => (scalar as i16) as u16 as u64,
        4 => (scalar as u32) as u64,
        8 => (scalar as i32 as i64) as u64,
        _ => 0,
    }
}

/// Sign-extend GPR scalar to i64 based on SEW. For vmslt_vx, vmseq_vx, etc.
#[inline]
pub(crate) fn scalar_sext(scalar: u32, sew_bytes: usize) -> i64 {
    match sew_bytes {
        1 => (scalar as i8) as i64,
        2 => (scalar as i16) as i64,
        4 => (scalar as i32) as i64,
        8 => scalar as i64,
        _ => 0,
    }
}

/// Signed mul-add: (src1 * src2 + dst) truncated to SEW. For vmacc.
#[inline]
pub(crate) fn binop_macc(src1: u64, src2: u64, dst: u64, sew: usize) -> u64 {
    match sew {
        1 => {
            let p = (src1 as i8 as i16).wrapping_mul(src2 as i8 as i16);
            (p.wrapping_add(dst as i8 as i16) as i8 as u8) as u64
        }
        2 => {
            let p = (src1 as i16 as i32).wrapping_mul(src2 as i16 as i32);
            (p.wrapping_add(dst as i16 as i32) as i16 as u16) as u64
        }
        4 => {
            let p = (src1 as i32 as i64).wrapping_mul(src2 as i32 as i64);
            (p.wrapping_add(dst as i32 as i64) as i32 as u32) as u64
        }
        8 => {
            let p = (src1 as i64).wrapping_mul(src2 as i64);
            p.wrapping_add(dst as i64) as u64
        }
        _ => dst,
    }
}
