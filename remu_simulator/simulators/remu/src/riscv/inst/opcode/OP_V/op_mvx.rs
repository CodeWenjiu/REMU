//! funct3 = 0b110: OP-MVX (vmv.s.x: vd[0] = rs1 scalar, rest unchanged)

use remu_types::isa::reg::{RegAccess, VectorCsrState, VrState};

use crate::riscv::inst::{DecodedInst, opcode::OP_V::OpMvxInst};

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    op: OpMvxInst,
) -> Result<(), remu_state::StateError> {
    match op {
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
