use remu_state::StateError;
use remu_types::isa::extension_v::VExtensionConfig;
use remu_types::isa::reg::{RegAccess, VectorCsrState, VrState};
use remu_types::isa::RvIsa;

use crate::riscv::inst::{funct3, rd, rs1, DecodedInst, Inst};

pub(crate) const OPCODE: u32 = 0b010_0111; // STORE-FP (0x27)
pub(crate) const INSTRUCTION_MIX: u32 = 10;

mod func3 {
    pub(super) const WIDTH_8: u32 = 0b000;
    /// vse32.v: EEW=32, unit-stride store
    pub(super) const WIDTH_32: u32 = 0b110;
}

mod lumop {
    pub(super) const VS1R: u32 = 0b01000;
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum StoreFpInst {
    Vs1r,
    /// vse8.v: store vl×8-bit elements from vs3 to mem[rs1 + i] (real instruction)
    Vse8,
    /// vse32.v: store vl×32-bit elements from vs3 to mem[rs1 + i*4]
    Vse32,
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

/// vs3 for unit-stride store: [24:20]. For vs1r, vs3 is in rd [11:7].
#[inline(always)]
fn vs3_store(inst: u32) -> u8 {
    ((inst >> 20) & 0x1F) as u8
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    if <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB > 0 {
        let f3 = funct3(inst);
        let store_fp = match f3 {
            func3::WIDTH_8 => {
                if mop(inst) != 0 || nf(inst) != 0 {
                    return DecodedInst::default();
                }
                let lum = lumop(inst);
                if lum == lumop::VS1R && vm(inst) == 1 {
                    StoreFpInst::Vs1r
                } else {
                    StoreFpInst::Vse8
                }
            }
            func3::WIDTH_32 if mop(inst) == 0 && nf(inst) == 0 => StoreFpInst::Vse32,
            _ => return DecodedInst::default(),
        };
        let (vs3, rs1_val) = match store_fp {
            StoreFpInst::Vs1r => (rd(inst), rs1(inst)),
            StoreFpInst::Vse8 | StoreFpInst::Vse32 => (vs3_store(inst), rs1(inst)),
        };
        return DecodedInst {
            rd: vs3,
            rs1: rs1_val,
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
        StoreFpInst::Vse8 => {
            if <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB > 0 {
                let state = ctx.state_mut();
                let vl = state.reg.csr.vector.vl();
                let vtype = state.reg.csr.vector.vtype();
                let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
                let vlmul = vtype & 0x7;
                let nf = match vlmul {
                    0 => 1,
                    1 => 2,
                    2 => 4,
                    3 => 8,
                    _ => 1,
                };
                let vs3 = decoded.rd as usize;
                let base = state.reg.gpr.raw_read(decoded.rs1.into());
                let n = vl.min((nf * vlenb) as u32);

                for i in 0..n {
                    let reg_i = (i as usize) / vlenb;
                    let off = (i as usize) % vlenb;
                    let chunk = state.reg.vr.raw_read(vs3 + reg_i);
                    state
                        .bus
                        .write_8(base.wrapping_add(i) as usize, chunk[off])
                        .map_err(StateError::from)?;
                }
                *state.reg.pc = state.reg.pc.wrapping_add(4);
            } else {
                unsafe { core::hint::unreachable_unchecked() }
            }
        }
        StoreFpInst::Vse32 => {
            if <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB > 0 {
                let state = ctx.state_mut();
                let vl = state.reg.csr.vector.vl();
                let vtype = state.reg.csr.vector.vtype();
                let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
                let vlmul = vtype & 0x7;
                let nf = match vlmul {
                    0 => 1,
                    1 => 2,
                    2 => 4,
                    3 => 8,
                    _ => 1,
                };
                let vs3 = decoded.rd as usize;
                let base = state.reg.gpr.raw_read(decoded.rs1.into());
                let n = vl.min((nf * vlenb / 4) as u32);

                for i in 0..n {
                    let reg_i = ((i as usize) * 4) / vlenb;
                    let off = ((i as usize) * 4) % vlenb;
                    let chunk = state.reg.vr.raw_read(vs3 + reg_i);
                    let val = u32::from_le_bytes(chunk[off..off + 4].try_into().unwrap());
                    state
                        .bus
                        .write_32(base.wrapping_add(i.wrapping_mul(4)) as usize, val)
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
