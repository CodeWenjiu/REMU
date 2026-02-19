//! LOAD-FP opcode (0x07): vector loads. vle8.v implemented.

use remu_state::StateError;
use remu_types::isa::extension_v::VExtensionConfig;
use remu_types::isa::reg::{RegAccess, VectorCsrState, VrState};
use remu_types::isa::RvIsa;

use crate::riscv::inst::{funct3, rd, rs1, DecodedInst, Inst};

pub(crate) const OPCODE: u32 = 0b000_0111; // LOAD-FP (0x07)
pub(crate) const INSTRUCTION_MIX: u32 = 10;

mod func3 {
    /// vle8.v: EEW=8, unit-stride load
    pub(super) const WIDTH_8: u32 = 0b000;
}

#[inline(always)]
fn mop(inst: u32) -> u32 {
    (inst >> 26) & 0x3
}

#[inline(always)]
fn nf(inst: u32) -> u32 {
    (inst >> 29) & 0x7
}

/// vd for unit-stride load is in rd [11:7] per RVV spec.
#[inline(always)]
fn vd_unit_stride(inst: u32) -> u8 {
    rd(inst)
}

/// Whole-register load: (inst & MASK) == MATCH (Spike encoding)
const MATCH_VL2RE16_V: u32 = 0x22805007;
const MATCH_VL2RE32_V: u32 = 0x22806007;
const MASK_VL2RE_V: u32 = 0xfff0707f;

#[derive(Clone, Copy, Debug)]
pub(crate) enum LoadFpInst {
    /// vle8.v: load vl×8-bit elements from mem[rs1 + i] into vd
    Vle8,
    /// vl2re16.v / vl2r.v (EEW=16): whole-reg load 2 regs, vd = rd, base = rs1
    Vl2re16,
    /// vl2re32.v: whole-reg load 2 regs, EEW=32 (Spike VI_LD_WHOLE)
    Vl2re32,
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    if <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB > 0 {
        let w = inst & MASK_VL2RE_V;
        if w == MATCH_VL2RE16_V || w == 0x22800007 {
            return DecodedInst {
                rd: vd_unit_stride(inst),
                rs1: rs1(inst),
                rs2: 0,
                imm: 0,
                inst: Inst::LoadFp(LoadFpInst::Vl2re16),
            };
        }
        if w == MATCH_VL2RE32_V {
            return DecodedInst {
                rd: vd_unit_stride(inst),
                rs1: rs1(inst),
                rs2: 0,
                imm: 0,
                inst: Inst::LoadFp(LoadFpInst::Vl2re32),
            };
        }
        let f3 = funct3(inst);
        let load_fp = match f3 {
            func3::WIDTH_8 if mop(inst) == 0 && nf(inst) == 0 => LoadFpInst::Vle8,
            _ => return DecodedInst::default(),
        };
        let vd = vd_unit_stride(inst);
        return DecodedInst {
            rd: vd,
            rs1: rs1(inst),
            rs2: 0,
            imm: 0,
            inst: Inst::LoadFp(load_fp),
        };
    }
    DecodedInst::default()
}

/// Max VLENB we support; used for stack buffer in vle8.
const MAX_VLENB: usize = 16;

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    let Inst::LoadFp(load_fp) = decoded.inst else { unreachable!() };

    match load_fp {
        LoadFpInst::Vl2re16 | LoadFpInst::Vl2re32 => {
            if <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB > 0 {
                let state = ctx.state_mut();
                let vlenb =
                    <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
                let vd = decoded.rd as usize;
                let base = state.reg.gpr.raw_read(decoded.rs1.into()) as usize;
                const NREGS: usize = 2;
                for r in 0..NREGS {
                    let mut chunk = vec![0u8; vlenb];
                    for j in 0..vlenb {
                        chunk[j] = state
                            .bus
                            .read_8(base.wrapping_add(r * vlenb).wrapping_add(j))
                            .map_err(StateError::from)?;
                    }
                    state.reg.vr.raw_write(vd + r, &chunk);
                }
                *state.reg.pc = state.reg.pc.wrapping_add(4);
            } else {
                unsafe { core::hint::unreachable_unchecked() }
            }
        }
        LoadFpInst::Vle8 => {
            if <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB > 0 {
                let state = ctx.state_mut();
                let vl = state.reg.csr.vector.vl();
                let vtype = state.reg.csr.vector.vtype();
                let vlenb =
                    <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
                let vlmul = vtype & 0x7;
                let nf = match vlmul {
                    0 => 1,
                    1 => 2,
                    2 => 4,
                    3 => 8,
                    _ => 1,
                };
                let vd = decoded.rd as usize;
                let base = state.reg.gpr.raw_read(decoded.rs1.into());
                let n = vl.min((nf * vlenb) as u32) as usize;

                let mut buf = [0u8; MAX_VLENB];
                let vlenb_use = vlenb.min(MAX_VLENB);
                let num_regs = (n + vlenb - 1) / vlenb;

                for reg_i in 0..num_regs {
                    let start = reg_i * vlenb;
                    let count = (n - start).min(vlenb);
                    buf[..vlenb_use].copy_from_slice(state.reg.vr.raw_read(vd + reg_i));
                    for j in 0..count {
                        buf[j] = state
                            .bus
                            .read_8(
                                base.wrapping_add(start as u32)
                                    .wrapping_add(j as u32) as usize,
                            )
                            .map_err(StateError::from)?;
                    }
                    state.reg.vr.raw_write(vd + reg_i, &buf[..vlenb]);
                }
                *state.reg.pc = state.reg.pc.wrapping_add(4);
            } else {
                unsafe { core::hint::unreachable_unchecked() }
            }
        }
    }

    Ok(())
}
