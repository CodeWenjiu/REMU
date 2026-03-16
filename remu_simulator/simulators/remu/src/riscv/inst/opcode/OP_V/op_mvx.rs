//! funct3 = 0b110: OP-MVX (vmv.s.x, vwmul.vx)

use remu_types::isa::reg::{RegAccess, VrState};

use crate::riscv::inst::{DecodedInst, opcode::OP_V::OpMvxInst};

use super::{mask_bit, VContext};

pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    op: OpMvxInst,
) -> Result<(), remu_state::StateError> {
    match op {
        OpMvxInst::Vwmul_vx => {
            let vctx = VContext::from_state::<P, C>(ctx);
            let state = ctx.state_mut();
            let nf_src = vctx.nf;
            let nf_dst = nf_src * 2;

            if vctx.sew_bytes >= 4 {
                return Err(remu_state::StateError::BusError(Box::new(
                    remu_state::bus::BusError::unmapped(0),
                )));
            }
            if vctx.vlmul == 3 {
                return Err(remu_state::StateError::BusError(Box::new(
                    remu_state::bus::BusError::unmapped(0),
                )));
            }
            let vd = decoded.rd as usize;
            let vs2 = decoded.rs2 as usize;
            if vd % nf_dst != 0 {
                return Err(remu_state::StateError::BusError(Box::new(
                    remu_state::bus::BusError::unmapped(0),
                )));
            }
            if vd + nf_dst > 32 || vs2 + nf_src > 32 {
                return Err(remu_state::StateError::BusError(Box::new(
                    remu_state::bus::BusError::unmapped(0),
                )));
            }
            if !(vd + nf_dst <= vs2 || vs2 + nf_src <= vd) {
                return Err(remu_state::StateError::BusError(Box::new(
                    remu_state::bus::BusError::unmapped(0),
                )));
            }
            if vd == 0 {
                return Err(remu_state::StateError::BusError(Box::new(
                    remu_state::bus::BusError::unmapped(0),
                )));
            }

            let vm = decoded.imm != 0;
            let v0 = state.reg.vr.raw_read(0).to_vec();
            let scalar_raw = state.reg.gpr.raw_read(decoded.rs1.into());
            let scalar_sext: i64 = match vctx.sew_bytes {
                1 => (scalar_raw as u8 as i8) as i64,
                2 => (scalar_raw as u16 as i16) as i64,
                _ => (scalar_raw as i32) as i64,
            };
            let total_elems_src = (nf_src * vctx.vlenb) / vctx.sew_bytes;
            let n = vctx.vl.min(total_elems_src as u32) as usize;
            let dst_bytes_per_elem = vctx.sew_bytes * 2;

            for r in 0..nf_dst {
                let mut dst_chunk = state.reg.vr.raw_read(vd + r).to_vec();
                let start_elem = (r * vctx.vlenb) / dst_bytes_per_elem;
                let end_elem = ((r + 1) * vctx.vlenb) / dst_bytes_per_elem;
                let loop_end = end_elem.min(n);
                for i in start_elem..loop_end {
                    if !vm && !mask_bit(&v0, i) {
                        continue;
                    }
                    let src_byte_off = i * vctx.sew_bytes;
                    let src_reg = src_byte_off / vctx.vlenb;
                    let src_off = src_byte_off % vctx.vlenb;
                    let src_chunk = state.reg.vr.raw_read(vs2 + src_reg);
                    let vs2_sext = match vctx.sew_bytes {
                        1 => (src_chunk[src_off] as i8 as i16) as i64,
                        2 => (i16::from_le_bytes(src_chunk[src_off..src_off + 2].try_into().unwrap()) as i32) as i64,
                        _ => i32::from_le_bytes(src_chunk[src_off..src_off + 4].try_into().unwrap()) as i64,
                    };
                    let prod = vs2_sext * scalar_sext;
                    let dst_byte_off = i * dst_bytes_per_elem;
                    let dst_off = dst_byte_off % vctx.vlenb;
                    match dst_bytes_per_elem {
                        2 => dst_chunk[dst_off..dst_off + 2].copy_from_slice(&(prod as i16).to_le_bytes()),
                        4 => dst_chunk[dst_off..dst_off + 4].copy_from_slice(&(prod as i32).to_le_bytes()),
                        8 => dst_chunk[dst_off..dst_off + 8].copy_from_slice(&prod.to_le_bytes()),
                        _ => {}
                    }
                }
                state.reg.vr.raw_write(vd + r, &dst_chunk);
            }
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
        OpMvxInst::Vmv_s_x => {
            let vctx = VContext::from_state::<P, C>(ctx);
            let state = ctx.state_mut();
            // vmv.s.x ignores vl and is unmasked; it always writes scalar to vd[0].
            let scalar = state.reg.gpr.raw_read(decoded.rs1.into());
            let mut chunk = state.reg.vr.raw_read(decoded.rd as usize).to_vec();
            vctx.sew.write(&mut chunk, 0, scalar as u64);
            state.reg.vr.raw_write(decoded.rd as usize, &chunk);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
    }
}
