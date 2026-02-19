//! funct3 = 0b011: OP-IVI

use remu_types::isa::{
    extension_v::VExtensionConfig,
    reg::{VectorCsrState, VrState},
    RvIsa,
};

use crate::riscv::inst::{DecodedInst, opcode::OP_V::OpIviInst};

use super::utils::{
    nf_from_vlmul, vector_element_loop, vector_mask_cmp_vi, VectorElementLoopMode,
};

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

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    op: OpIviInst,
) -> Result<(), remu_state::StateError> {
    match op {
        OpIviInst::Vmerge_vim => {
            let simm5 = ((decoded.imm << 27) as i32) >> 27;
            let vm = decoded.rs1 != 0;
            let mode = if vm {
                VectorElementLoopMode::Unmasked
            } else {
                VectorElementLoopMode::Masked
            };
            vector_element_loop(
                ctx,
                decoded.rd as usize,
                if vm { None } else { Some(decoded.rs2 as usize) },
                mode,
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
        OpIviInst::Vmseq_vi => {
            vector_mask_cmp_vi::<P, C>(ctx, decoded.rd as usize, decoded.rs2 as usize, decoded.imm)
        }
        OpIviInst::Vmv1r_v => {
            let state = ctx.state_mut();
            let vs2 = decoded.rs2 as usize;
            let vd = decoded.rd as usize;
            let data = state.reg.vr.raw_read(vs2).to_vec();
            state.reg.vr.raw_write(vd, &data);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
        OpIviInst::Vrsub_vi => {
            let simm5 = ((decoded.imm << 27) as i32) >> 27;
            let vm = decoded.rs1 != 0;
            let mode = if vm {
                VectorElementLoopMode::Unmasked
            } else {
                VectorElementLoopMode::Masked
            };
            vector_element_loop(
                ctx,
                decoded.rd as usize,
                Some(decoded.rs2 as usize),
                mode,
                |_, sew, src, mask, dst| {
                    if mask {
                        let v = src.unwrap_or(0);
                        match sew {
                            1 => (simm5 as i8).wrapping_sub(v as i8) as u8 as u64,
                            2 => (simm5 as i16).wrapping_sub(v as i16) as u16 as u64,
                            4 => simm5.wrapping_sub(v as i32) as u32 as u64,
                            8 => (simm5 as i64).wrapping_sub(v as i64) as u64,
                            _ => 0,
                        }
                    } else {
                        dst
                    }
                },
            )
        }
        OpIviInst::Vadd_vi => {
            let simm5 = ((decoded.imm << 27) as i32) >> 27;
            let vm = decoded.rs1 != 0;
            let mode = if vm {
                VectorElementLoopMode::Unmasked
            } else {
                VectorElementLoopMode::Masked
            };
            vector_element_loop(
                ctx,
                decoded.rd as usize,
                Some(decoded.rs2 as usize),
                mode,
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
        OpIviInst::Vslidedown_vi => {
            vector_slidedown_vi::<P, C>(
                ctx,
                decoded.rd as usize,
                decoded.rs2 as usize,
                decoded.imm,
            )
        }
    }
}
