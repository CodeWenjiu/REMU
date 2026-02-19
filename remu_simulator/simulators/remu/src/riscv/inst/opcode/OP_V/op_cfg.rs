//! funct3 = 0b111: vsetivli, vsetvli

use remu_types::isa::{
    extension_v::VExtensionConfig,
    reg::{RegAccess, VectorCsrState},
    RvIsa,
};

use crate::riscv::inst::{DecodedInst, opcode::OP_V::OpCfgInst};

#[inline(always)]
fn zimm_to_vtype(zimm: u32) -> u32 {
    zimm & 0xFF
}

#[inline(always)]
fn calculate_vlmax(vlenb: u32, vtype: u32) -> u32 {
    let vsew = (vtype >> 3) & 0x7;
    let vlmul = vtype & 0x7;
    let lmul_shift: i8 = match vlmul {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        5 => -3,
        6 => -2,
        7 => -1,
        _ => return 0,
    };
    let sew_shift: i8 = match vsew {
        0..=3 => -(vsew as i8),
        _ => return 0,
    };
    let total_shift = lmul_shift + sew_shift;
    if total_shift >= 0 {
        vlenb << total_shift
    } else {
        vlenb >> (-total_shift)
    }
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    op: OpCfgInst,
) -> Result<(), remu_state::StateError> {
    match op {
        OpCfgInst::Vsetivli => {
            let zimm = (decoded.imm >> 5) & 0x3FF;
            let vtype = zimm_to_vtype(zimm);
            let uimm = decoded.imm & 0x1F;
            let rd = decoded.rd;
            let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB;
            let vlmax = calculate_vlmax(vlenb, vtype);
            let vl = uimm.min(vlmax);
            let state = ctx.state_mut();
            state.reg.csr.vector.set_vtype(vtype);
            state.reg.csr.vector.set_vl(vl);
            state.reg.gpr.raw_write(rd.into(), vl);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
        OpCfgInst::Vsetvli => {
            let zimm = (decoded.imm >> 5) & 0x3FF;
            let vtype = zimm_to_vtype(zimm);
            let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB;
            let vlmax = calculate_vlmax(vlenb, vtype);
            let rd = decoded.rd;
            let rs1 = decoded.rs1;
            let state = ctx.state_mut();
            state.reg.csr.vector.set_vtype(vtype);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            if rs1 == 0 && rd == 0 {
                return Ok(());
            }
            let avl = if rs1 == 0 {
                u32::MAX
            } else {
                state.reg.gpr.raw_read(rs1.into())
            };
            let vl = avl.min(vlmax);
            state.reg.csr.vector.set_vl(vl);
            state.reg.gpr.raw_write(rd.into(), vl);
            Ok(())
        }
    }
}
