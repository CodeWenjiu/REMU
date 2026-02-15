//! RISC-V V extension (OP-V opcode 0x57). Decode only when VLENB > 0 (see inst/mod.rs).
//! Match on (funct3, top2) for decode; illegal encodings return DecodedInst::default().

use remu_types::isa::extension_v::VExtensionConfig;
use remu_types::isa::reg::{RegAccess, VectorCsrState};
use remu_types::isa::RvIsa;

use crate::riscv::inst::{funct3, rd, rs1, rs2, v_funct6, v_vm, DecodedInst, Inst};

pub(crate) const OPCODE: u32 = 0b101_0111; // OP-V
pub(crate) const INSTRUCTION_MIX: u32 = 5;

/// V extension instruction kind; Inst::V carries this.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VInst {
    Vsetivli,
}

/// vsetivli uses standard V layout: v_rd=rd, v_rs1=uimm(AVL), zimm = v_funct6[3:0] || v_vm || v_rs2 (inst[29:20]).
const VSETIVLI_FUNCT3: u32 = 0b111;
const VSETIVLI_TOP2: u32 = 0b11; // inst[31:30] == 11

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
        (VSETIVLI_FUNCT3, VSETIVLI_TOP2) => VInst::Vsetivli,
        _ => return DecodedInst::default(),
    };

    match v_inst {
        VInst::Vsetivli => {
            let v_rd = rd(inst);
            let uimm = rs1(inst) as u32;
            let zimm = v_zimm_vsetivli(inst);
            // imm: [17:8] = zimm[9:0]; [4:0] = uimm[4:0]
            DecodedInst {
                rd: v_rd,
                rs1: 0,
                rs2: 0,
                imm: (zimm << 8) | uimm,
                inst: Inst::V(VInst::Vsetivli),
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
    }
}
