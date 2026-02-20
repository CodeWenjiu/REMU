//! funct3 = 0b110: OP-MVX (vmv.s.x, vwmul.vx)

use remu_types::isa::{
    extension_v::VExtensionConfig,
    reg::{RegAccess, VectorCsrState, VrState},
    RvIsa,
};

use crate::riscv::inst::{DecodedInst, opcode::OP_V::OpMvxInst};

use super::utils::nf_from_vlmul;

pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    op: OpMvxInst,
) -> Result<(), remu_state::StateError> {
    match op {
        OpMvxInst::Vwmul_vx => {
            let state = ctx.state_mut();
            let vl = state.reg.csr.vector.vl();
            let vtype = state.reg.csr.vector.vtype();
            let vlmul = vtype & 0x7;
            let vsew = (vtype >> 3) & 0x7;
            let sew_bytes = 1 << (vsew & 0x3);
            let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
            let nf_src = nf_from_vlmul(vlmul);
            let nf_dst = nf_src * 2;

            if sew_bytes >= 4 {
                return Err(remu_state::StateError::BusError(Box::new(
                    remu_state::bus::BusError::unmapped(0),
                )));
            }
            if vlmul == 3 {
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
            let scalar_sext: i64 = match sew_bytes {
                1 => (scalar_raw as u8 as i8) as i64,
                2 => (scalar_raw as u16 as i16) as i64,
                _ => (scalar_raw as i32) as i64,
            };
            let total_elems_src = (nf_src * vlenb) / sew_bytes;
            let n = vl.min(total_elems_src as u32) as usize;
            let dst_bytes_per_elem = sew_bytes * 2;

            for r in 0..nf_dst {
                let mut dst_chunk = state.reg.vr.raw_read(vd + r).to_vec();
                let start_elem = (r * vlenb) / dst_bytes_per_elem;
                let end_elem = ((r + 1) * vlenb) / dst_bytes_per_elem;
                let loop_end = end_elem.min(n);
                for i in start_elem..loop_end {
                    if !vm && ((v0[i / 8] >> (i % 8)) & 1 == 0) {
                        continue;
                    }
                    let src_byte_off = i * sew_bytes;
                    let src_reg = src_byte_off / vlenb;
                    let src_off = src_byte_off % vlenb;
                    let src_chunk = state.reg.vr.raw_read(vs2 + src_reg);
                    let vs2_sext = match sew_bytes {
                        1 => (src_chunk[src_off] as i8 as i16) as i64,
                        2 => (i16::from_le_bytes(src_chunk[src_off..src_off + 2].try_into().unwrap()) as i32) as i64,
                        _ => i32::from_le_bytes(src_chunk[src_off..src_off + 4].try_into().unwrap()) as i64,
                    };
                    let prod = vs2_sext * scalar_sext;
                    let dst_byte_off = i * dst_bytes_per_elem;
                    let dst_off = dst_byte_off % vlenb;
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
            let state = ctx.state_mut();
            // vmv.s.x ignores vl and is unmasked; it always writes scalar to vd[0].
            let vtype = state.reg.csr.vector.vtype();
            let vsew = (vtype >> 3) & 0x7;
            let sew_bytes = 1 << (vsew & 0x3);
            let scalar = state.reg.gpr.raw_read(decoded.rs1.into());
            let mut chunk = state.reg.vr.raw_read(decoded.rd as usize).to_vec();
            match sew_bytes {
                1 => chunk[0] = scalar as u8,
                2 => chunk[0..2].copy_from_slice(&(scalar as u16).to_le_bytes()),
                4 => chunk[0..4].copy_from_slice(&(scalar as u32).to_le_bytes()),
                8 => chunk[0..8].copy_from_slice(&(scalar as u64).to_le_bytes()),
                _ => {}
            }
            state.reg.vr.raw_write(decoded.rd as usize, &chunk);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
    }
}
