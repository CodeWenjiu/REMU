//! RISC-V V extension (OP-V opcode 0x57). Decode only when VLENB > 0 (see inst/mod.rs).
//! Match on (funct3, top2) for decode; illegal encodings return DecodedInst::default().

use remu_types::isa::extension_v::VExtensionConfig;
use remu_types::isa::reg::{RegAccess, VectorCsrState, VrState};
use remu_types::isa::RvIsa;

use crate::riscv::inst::{DecodedInst, Inst, funct3, rd, rs1, rs2, v_funct6, v_vm};

pub(crate) const OPCODE: u32 = 0b101_0111; // OP-V
pub(crate) const INSTRUCTION_MIX: u32 = 5;

mod func3 {
    /// vsetivli: funct3=111.
    pub(super) const VSETIVLI: u32 = 0b111;
    /// vid.v: funct3=010 (OPIVI/OPMV form, used by Spike/ref).
    pub(super) const VID_V: u32 = 0b010;
    /// vrsub.vi: funct3=011 (OPIVI, Spike MATCH_VRSUB_VI).
    pub(super) const RSUB_VI: u32 = 0b011;
}

/// inst[31:30], used with funct3 to identify V instruction form.
mod top2 {
    /// vsetivli: 11.
    pub(super) const VSETIVLI: u32 = 0b11;
    /// vid.v: 01.
    pub(super) const VID_V: u32 = 0b01;
    /// vrsub.vi: 00 (funct6=0x03).
    pub(super) const RSUB_VI: u32 = 0b00;
}

mod funct6 {
    /// vid.v: 0x14 (matches Spike/ref encoding for 0x5208a457).
    pub(super) const VID_V: u32 = 0x14;
    /// vrsub.vi: 0x03 (OPIVI add/sub group).
    pub(super) const RSUB_VI: u32 = 0x03;
}

/// V extension instruction kind; Inst::V carries this.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub(crate) enum VInst {
    Vsetivli,
    /// vid.v: write element indices 0..vl to vd (RVV 1.0).
    Vid_v,
    /// vrsub.vi: vd[i] = simm5 - vs2[i] (RVV 1.0, OPIVI).
    Vrsub_vi,
}

/// zimm[9:0] for vsetivli = v_funct6[3:0] || v_vm || v_rs2 (standard V fields combined).
#[inline(always)]
fn v_zimm_vsetivli(inst: u32) -> u32 {
    ((v_funct6(inst) & 0xF) << 6) | (v_vm(inst) << 5) | rs2(inst) as u32
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);
    let top2 = (inst >> 30) & 0x3;

    let v_inst = match (f3, top2) {
        (func3::VSETIVLI, top2::VSETIVLI) => VInst::Vsetivli,
        (func3::VID_V, top2::VID_V) => {
            if v_funct6(inst) == funct6::VID_V && v_vm(inst) == 1 && rs2(inst) == 0 {
                VInst::Vid_v
            } else {
                return DecodedInst::default();
            }
        }
        (func3::RSUB_VI, top2::RSUB_VI) => {
            if v_funct6(inst) == funct6::RSUB_VI {
                VInst::Vrsub_vi
            } else {
                return DecodedInst::default();
            }
        }
        _ => return DecodedInst::default(),
    };

    match v_inst {
        VInst::Vsetivli => {
            let uimm = rs1(inst) as u32;
            let zimm = v_zimm_vsetivli(inst);
            // imm: [17:8] = zimm[9:0]; [4:0] = uimm[4:0]
            DecodedInst {
                rd: rd(inst),
                rs1: 0,
                rs2: 0,
                imm: (zimm << 8) | uimm,
                inst: Inst::V(VInst::Vsetivli),
            }
        }
        VInst::Vid_v => DecodedInst {
            rd: rd(inst),
            rs1: 0,
            rs2: 0,
            imm: 0,
            inst: Inst::V(VInst::Vid_v),
        },
        VInst::Vrsub_vi => {
            let simm5 = ((rs1(inst) as i32) << 27 >> 27) as u32;
            DecodedInst {
                rd: rd(inst),
                rs1: 0,
                rs2: rs2(inst),
                imm: simm5,
                inst: Inst::V(VInst::Vrsub_vi),
            }
        }
    }
}

/// zimm[9:0] from vsetivli has same layout as vtype CSR lower 8 bits per RVV 1.0:
/// vtype[2:0]=vlmul, vtype[5:3]=vsew, vtype[6]=vta, vtype[7]=vma. vill=0.
#[inline(always)]
fn zimm_to_vtype(zimm: u32) -> u32 {
    (zimm & 0xFF) & !(1 << 31)
}

#[inline(always)]
fn vlmax_vlenb_vtype(vlenb: u32, vtype: u32) -> u32 {
    let vlen = vlenb * 8;
    let vsew = (vtype >> 3) & 0x7;
    let vlmul = vtype & 0x7;
    let sew = match vsew {
        0 => 8,
        1 => 16,
        2 => 32,
        3 => 64,
        _ => 8,
    };
    let (num, denom) = match vlmul {
        0 => (1, 1),
        1 => (2, 1),
        2 => (4, 1),
        3 => (8, 1),
        4 => (1, 2),
        5 => (1, 4),
        6 => (1, 8),
        _ => (1, 1),
    };
    ((vlen / sew) * num) / denom
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    let v = match decoded.inst {
        Inst::V(v) => v,
        _ => return crate::riscv::inst::opcode::UNKNOWN::execute::<P, C>(ctx, decoded),
    };

    match v {
        VInst::Vsetivli => {
            let zimm = (decoded.imm >> 8) & 0x3FF;
            let uimm = decoded.imm & 0x1F;
            let rd = decoded.rd;
            let vtype = zimm_to_vtype(zimm);
            let vlenb = <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB;
            let vlmax = vlmax_vlenb_vtype(vlenb, vtype);
            let vl = uimm.min(vlmax);

            let state = ctx.state_mut();
            state.reg.csr.vector.set_vtype(vtype);
            state.reg.csr.vector.set_vl(vl);
            state.reg.gpr.raw_write(rd.into(), vl);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
        VInst::Vid_v => {
            let state = ctx.state_mut();
            let vl = state.reg.csr.vector.vl();
            let vtype = state.reg.csr.vector.vtype();
            let vsew = (vtype >> 3) & 0x7;
            let sew_bytes = match vsew {
                0 => 1,
                1 => 2,
                2 => 4,
                3 => 8,
                _ => 1,
            };
            let vlenb =
                <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
            let mut buf = vec![0u8; vlenb];
            for i in 0..vl {
                let idx = i as u32;
                let off = (i as usize) * sew_bytes;
                match sew_bytes {
                    1 => buf[off] = idx as u8,
                    2 => buf[off..off + 2].copy_from_slice(&(idx as u16).to_le_bytes()),
                    4 => buf[off..off + 4].copy_from_slice(&idx.to_le_bytes()),
                    8 => buf[off..off + 8].copy_from_slice(&(idx as u64).to_le_bytes()),
                    _ => {}
                }
            }
            state.reg.vr.raw_write(decoded.rd.into(), &buf);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
        VInst::Vrsub_vi => {
            let state = ctx.state_mut();
            let vl = state.reg.csr.vector.vl();
            let vtype = state.reg.csr.vector.vtype();
            let vsew = (vtype >> 3) & 0x7;
            let sew_bytes = match vsew {
                0 => 1,
                1 => 2,
                2 => 4,
                3 => 8,
                _ => 1,
            };
            let vlenb =
                <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB as usize;
            let simm5 = decoded.imm as i32;
            let vs2_slice = state.reg.vr.raw_read(decoded.rs2.into());
            let mut buf = vec![0u8; vlenb];
            for i in 0..vl {
                let off = (i as usize) * sew_bytes;
                match sew_bytes {
                    1 => {
                        let a = simm5 as i8;
                        let b = vs2_slice[off] as i8;
                        buf[off] = (a.wrapping_sub(b)) as u8;
                    }
                    2 => {
                        let a = simm5 as i16;
                        let b = u16::from_le_bytes([vs2_slice[off], vs2_slice[off + 1]]) as i16;
                        buf[off..off + 2].copy_from_slice(&(a.wrapping_sub(b) as u16).to_le_bytes());
                    }
                    4 => {
                        let a = simm5;
                        let b =
                            i32::from_le_bytes(vs2_slice[off..off + 4].try_into().unwrap());
                        buf[off..off + 4].copy_from_slice(&(a.wrapping_sub(b) as u32).to_le_bytes());
                    }
                    8 => {
                        let a = simm5 as i64;
                        let b =
                            i64::from_le_bytes(vs2_slice[off..off + 8].try_into().unwrap());
                        buf[off..off + 8].copy_from_slice(&(a.wrapping_sub(b) as u64).to_le_bytes());
                    }
                    _ => {}
                }
            }
            state.reg.vr.raw_write(decoded.rd.into(), &buf);
            *state.reg.pc = state.reg.pc.wrapping_add(4);
            Ok(())
        }
    }
}
