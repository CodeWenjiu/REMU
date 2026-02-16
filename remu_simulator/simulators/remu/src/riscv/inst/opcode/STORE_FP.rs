use remu_state::StateError;
use remu_types::isa::extension_v::VExtensionConfig;
use remu_types::isa::reg::{RegAccess, VectorCsrState, VrState};
use remu_types::isa::RvIsa;

use crate::riscv::inst::{funct3, rd, rs1, DecodedInst, Inst};

pub(crate) const OPCODE: u32 = 0b010_0111; // STORE-FP (0x27)
pub(crate) const INSTRUCTION_MIX: u32 = 10;

mod func3 {
    pub(super) const WIDTH_8: u32 = 0b000;
}

mod lumop {
    pub(super) const VS1R: u32 = 0b01000;
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum StoreFpInst {
    Vs1r,
}

#[inline(always)]
fn lumop(inst: u32) -> u32 {
    (inst >> 20) & 0x1F
}

#[inline(always)]
fn vm(inst: u32) -> u32 {
    (inst >> 25) & 1
}

#[inline(always)]
fn mop(inst: u32) -> u32 {
    (inst >> 26) & 0x3
}

#[inline(always)]
fn nf(inst: u32) -> u32 {
    (inst >> 29) & 0x7
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    if <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB > 0 {
        let f3 = funct3(inst);
        let store_fp = match f3 {
            func3::WIDTH_8 => {
                let lum = lumop(inst);
                // vs1r.v: lumop=8, vm=1, mop=0, nf=0
                if lum == lumop::VS1R && vm(inst) == 1 && mop(inst) == 0 && nf(inst) == 0 {
                    StoreFpInst::Vs1r
                } else {
                    return DecodedInst::default();
                }
            }
            _ => return DecodedInst::default(),
        };
        return DecodedInst {
            // For vs1r.v, vs3 data is in 'rd' field
            rd: rd(inst), // vs3
            rs1: rs1(inst),
            rs2: 0,
            imm: 0,
            inst: Inst::StoreFp(store_fp),
        };
    }
    DecodedInst::default()
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    let Inst::StoreFp(store) = decoded.inst else { unreachable!() };

    match store {
        StoreFpInst::Vs1r => {
            if <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB > 0 {
                // Whole register store ignores vtype and vl, but requires vstart == 0
                if ctx.state_mut().reg.csr.vector.vstart() != 0 {
                    return crate::riscv::inst::opcode::UNKNOWN::execute(ctx, decoded);
                }

                let state = ctx.state_mut();
                let rs1_val = state.reg.gpr.raw_read(decoded.rs1.into());

                // vs3 is stored in decoded.rd
                let data = state.reg.vr.raw_read(decoded.rd.into());

                // Write VLENB bytes to memory at rs1_val
                for (i, &byte) in data.iter().enumerate() {
                    state
                        .bus
                        .write_8(rs1_val.wrapping_add(i as u32) as usize, byte)
                        .map_err(StateError::from)?;
                }
                *state.reg.pc = state.reg.pc.wrapping_add(4);
            } else {
                unsafe { core::hint::unreachable_unchecked() }
            }
        }
    }

    Ok(())
}
