//! Custom opcode **CUS0** (`0b0001011` / `0x0B`, RV custom-0) + simulated MNIST accelerator.
//!
//! - **NN_LOAD_ACT** — buffer one input activation (`rs1` / `rs2` GPR values).
//! - **NN_START** — run embedded MLP forward on the buffer.
//! - **NN_LOAD** — read one logit; **`rs1`** = GPR holding output index, **`rd`** = destination.

mod mnist_infer;

use core::hint::unreachable_unchecked;

use remu_state::StateError;
use remu_types::isa::reg::RegAccess;

use crate::riscv::inst::{funct3, funct7, imm_i, rd, rs1, rs2, DecodedInst, Inst};

pub(crate) const OPCODE: u32 = 0b000_1011;
pub(crate) const INSTRUCTION_MIX: u32 = 60;

/// `NN_START`: only `opcode` + `funct3`, all other bits cleared.
const ENCODE_NN_START: u32 = (0b001_u32 << 12) | OPCODE;

mod func3 {
    pub(super) const NN_LOAD_ACT: u32 = 0b000;
    pub(super) const CTL: u32 = 0b001;
    pub(super) const NN_LOAD_RD: u32 = 0b010;
}

/// Top-level CUS0 decoded operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Cus0Inst {
    /// `func3=000`, `funct7=0` — R-type: `rs1` / `rs2` operand registers.
    NnLoadAct,
    /// `func3=001` — CTL family.
    Ctl(CtlInst),
    /// `func3=010` — operations that use `rd` (I-type layout for decode).
    NnLoadRd(NnLoadRdInst),
}

/// `func3=001` — no source operands; extra bits will select variants later.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CtlInst {
    /// All non-fixed fields zero (`0x100B`).
    NnStart,
}

/// `func3=010` — `rd` present; **`rs1`** carries extract index (GPR number).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NnLoadRdInst {
    /// I-type: `imm_i=0`, `simm12=0`; **`rd`** = destination, **`rs1`** = bias index register.
    NnLoad,
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    match funct3(inst) {
        func3::NN_LOAD_ACT => {
            if funct7(inst) != 0 {
                return DecodedInst::default();
            }
            DecodedInst {
                rd: rd(inst),
                rs1: rs1(inst),
                rs2: rs2(inst),
                imm: 0,
                inst: Inst::Cus0(Cus0Inst::NnLoadAct),
            }
        }
        func3::CTL => {
            if inst == ENCODE_NN_START {
                DecodedInst {
                    rd: 0,
                    rs1: 0,
                    rs2: 0,
                    imm: 0,
                    inst: Inst::Cus0(Cus0Inst::Ctl(CtlInst::NnStart)),
                }
            } else {
                DecodedInst::default()
            }
        }
        func3::NN_LOAD_RD => {
            if imm_i(inst) == 0 {
                DecodedInst {
                    rd: rd(inst),
                    rs1: rs1(inst),
                    rs2: 0,
                    imm: 0,
                    inst: Inst::Cus0(Cus0Inst::NnLoadRd(NnLoadRdInst::NnLoad)),
                }
            } else {
                DecodedInst::default()
            }
        }
        _ => DecodedInst::default(),
    }
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
) -> Result<(), StateError> {
    let state = ctx.state_mut();
    let Inst::Cus0(op) = decoded.inst else {
        unsafe { unreachable_unchecked() }
    };
    match op {
        Cus0Inst::NnLoadAct => {
            let idx = state.reg.gpr.raw_read(decoded.rs1.into()) as i32;
            let v = state.reg.gpr.raw_read(decoded.rs2.into()) as i32;
            if idx >= 0 {
                let u = idx as usize;
                mnist_infer::buffer_load_act(u, v as i8);
            }
        }
        Cus0Inst::Ctl(CtlInst::NnStart) => {
            mnist_infer::run_inference();
        }
        Cus0Inst::NnLoadRd(NnLoadRdInst::NnLoad) => {
            let k = state.reg.gpr.raw_read(decoded.rs1.into()) as i32;
            let idx = k.clamp(0, 9) as usize;
            let logit = mnist_infer::read_logit(idx);
            if decoded.rd != 0 {
                state.reg.gpr.raw_write(decoded.rd.into(), logit as u32);
            }
        }
    }
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}
