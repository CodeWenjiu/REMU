//! funct3 = 0b111: vsetivli, vsetvli

use remu_types::isa::{
    extension_v::VExtensionConfig,
    reg::{RegAccess, VectorCsrState},
    RvIsa,
};

use crate::riscv::inst::{DecodedInst, opcode::OP_V::OpCfgInst};

use super::calculate_vlmax;

fn zimm_to_vtype(zimm: u32) -> u32 {
    zimm & 0xFF
}

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
