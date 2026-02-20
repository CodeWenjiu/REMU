//! LOAD-FP opcode (0x07): vector loads. vle8.v implemented.

use remu_state::StateError;
use remu_types::isa::extension_v::VExtensionConfig;
use remu_types::isa::reg::{RegAccess, VectorCsrState, VrState};
use remu_types::isa::RvIsa;

use crate::riscv::inst::{funct3, rd, rs1, rs2, DecodedInst, Inst};

pub(crate) const OPCODE: u32 = 0b000_0111; // LOAD-FP (0x07)
pub(crate) const INSTRUCTION_MIX: u32 = 10;

mod func3 {
    /// vle8.v: EEW=8, unit-stride load
    pub(super) const WIDTH_8: u32 = 0b000;
    /// vl1re16.v / vle16.v: EEW=16 (funct3=101)
    pub(super) const WIDTH_16: u32 = 0b101;
    /// vlse32.v: EEW=32, strided load (funct3=110 per RVV encoding)
    pub(super) const WIDTH_32: u32 = 0b110;
}

/// lumop (Load unit-stride mask op): bit[24:20]. 0x08 = VL1R (whole-register load 1 reg).
mod lumop {
    pub(super) const VL1R: u32 = 0b01000;
}

#[inline(always)]
fn mop(inst: u32) -> u32 {
    (inst >> 26) & 0x3
}

#[inline(always)]
fn nf(inst: u32) -> u32 {
    (inst >> 29) & 0x7
}

/// vm (mask): 1 = unmasked, 0 = masked (use v0)
#[inline(always)]
fn vm(inst: u32) -> u32 {
    (inst >> 25) & 1
}

/// lumop: bit[24:20] (unit-stride load/store extra opcode).
#[inline(always)]
fn lumop(inst: u32) -> u32 {
    (inst >> 20) & 0x1F
}

/// vd for unit-stride load is in rd [11:7] per RVV spec.
#[inline(always)]
fn vd_unit_stride(inst: u32) -> u8 {
    rd(inst)
}

/// Whole-register load: (inst & MASK) == MATCH (Spike encoding, for vl2re*)
const MATCH_VL2RE16_V: u32 = 0x22805007;
const MATCH_VL2RE32_V: u32 = 0x22806007;
const MASK_VL2RE_V: u32 = 0xfff0707f;

#[derive(Clone, Copy, Debug)]
pub(crate) enum LoadFpInst {
    /// vle8.v: load vl×8-bit elements from mem[rs1 + i] into vd
    Vle8,
    /// vlseg4e8.v: unit-stride segment load 4×8-bit, de-interleave into vd..vd+3
    Vlseg4e8,
    /// vl1re16.v: whole-reg load 1 reg, EEW=16, vd = rd, base = rs1 (ignores vl/vtype)
    Vl1re16,
    /// vl2re16.v / vl2r.v (EEW=16): whole-reg load 2 regs, vd = rd, base = rs1
    Vl2re16,
    /// vl2re32.v: whole-reg load 2 regs, EEW=32 (Spike VI_LD_WHOLE)
    Vl2re32,
    /// vlse32.v: strided load, vd[i] = mem[rs1 + i * rs2], EEW=32, rs2 = stride in bytes (GPR)
    Vlse32,
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    if <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB > 0 {
        let f3 = funct3(inst);
        let m = mop(inst);
        let nf_val = nf(inst);
        // 整寄存器加载：mop=0, nf=0, vm=1, lumop=VL1R；按 funct3 区分 EEW，目前只实现 e16
        if m == 0 && nf_val == 0 && vm(inst) == 1 && lumop(inst) == lumop::VL1R {
            if f3 == func3::WIDTH_16 {
                return DecodedInst {
                    rd: vd_unit_stride(inst),
                    rs1: rs1(inst),
                    rs2: 0,
                    imm: 0,
                    inst: Inst::LoadFp(LoadFpInst::Vl1re16),
                };
            }
            // vl1re8 / vl1re32 等可在此按 f3 扩展
        }
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
        let load_fp = match (f3, m, nf_val) {
            (func3::WIDTH_8, 0, 0) => LoadFpInst::Vle8,
            (func3::WIDTH_8, 0, 3) => LoadFpInst::Vlseg4e8, // nf=3 -> 4 fields
            (func3::WIDTH_32, 2, 0) => LoadFpInst::Vlse32,  // mop=2 strided, nf=0
            _ => return DecodedInst::default(),
        };
        let vd = vd_unit_stride(inst);
        return DecodedInst {
            rd: vd,
            rs1: rs1(inst),
            rs2: if matches!(load_fp, LoadFpInst::Vlse32) {
                rs2(inst)
            } else {
                0
            },
            imm: vm(inst),
            inst: Inst::LoadFp(load_fp),
        };
    }
    DecodedInst::default()
}

/// Max VLENB we support; used for stack buffer in vle8.
const MAX_VLENB: usize = 16;

pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    let Inst::LoadFp(load_fp) = decoded.inst else { unreachable!() };

    match load_fp {
        LoadFpInst::Vlseg4e8 => {
            if <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB > 0 {
                let state = ctx.state_mut();
                let vl = state.reg.csr.vector.vl();
                let vlenb =
                    <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
                let n = vl.min(vlenb as u32) as usize;
                let vd = decoded.rd as usize;
                let base = state.reg.gpr.raw_read(decoded.rs1.into());
                let vm = (decoded.imm & 1) != 0;
                let v0 = state.reg.vr.raw_read(0).to_vec();

                let mut vd_buf: Vec<Vec<u8>> =
                    (0..4).map(|r| state.reg.vr.raw_read(vd + r).to_vec()).collect();
                for i in 0..n {
                    let active = vm || ((v0[i / 8] >> (i % 8)) & 1 != 0);
                    if !active {
                        continue;
                    }
                    for f in 0..4 {
                        let addr =
                            base.wrapping_add((i * 4 + f) as u32) as usize;
                        let val = state.bus.read_8(addr).map_err(StateError::from)?;
                        vd_buf[f][i] = val;
                    }
                }
                for f in 0..4 {
                    state.reg.vr.raw_write(vd + f, &vd_buf[f]);
                }
                *state.reg.pc = state.reg.pc.wrapping_add(4);
            } else {
                unsafe { core::hint::unreachable_unchecked() }
            }
        }
        LoadFpInst::Vl1re16 => {
            if <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB > 0 {
                let state = ctx.state_mut();
                let vlenb =
                    <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
                let vd = decoded.rd as usize;
                let base = state.reg.gpr.raw_read(decoded.rs1.into()) as usize;
                let mut chunk = vec![0u8; vlenb];
                for j in 0..vlenb {
                    chunk[j] = state
                        .bus
                        .read_8(base.wrapping_add(j))
                        .map_err(StateError::from)?;
                }
                state.reg.vr.raw_write(vd, &chunk);
                *state.reg.pc = state.reg.pc.wrapping_add(4);
            } else {
                unsafe { core::hint::unreachable_unchecked() }
            }
        }
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
        LoadFpInst::Vlse32 => {
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
                const SEW_BYTES: usize = 4;
                let vd = decoded.rd as usize;
                let base = state.reg.gpr.raw_read(decoded.rs1.into());
                let stride = state.reg.gpr.raw_read(decoded.rs2.into());
                let vm = (decoded.imm & 1) != 0;
                let v0 = state.reg.vr.raw_read(0).to_vec();
                let n = vl.min((nf * vlenb / SEW_BYTES) as u32) as usize;
                if vd + nf > 32 {
                    return Err(StateError::BusError(Box::new(
                        remu_state::bus::BusError::unmapped(0),
                    )));
                }
                for r in 0..nf {
                    let mut dst_chunk = state.reg.vr.raw_read(vd + r).to_vec();
                    let start_elem = (r * vlenb) / SEW_BYTES;
                    let end_elem = ((r + 1) * vlenb) / SEW_BYTES;
                    let loop_end = end_elem.min(n);
                    for i in start_elem..loop_end {
                        let active = vm || ((v0[i / 8] >> (i % 8)) & 1 != 0);
                        if !active {
                            continue;
                        }
                        let addr = base
                            .wrapping_add((i as u32).wrapping_mul(stride))
                            as usize;
                        let val = state.bus.read_32(addr).map_err(StateError::from)?;
                        let off = (i * SEW_BYTES) % vlenb;
                        dst_chunk[off..off + SEW_BYTES]
                            .copy_from_slice(&val.to_le_bytes());
                    }
                    state.reg.vr.raw_write(vd + r, &dst_chunk);
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
