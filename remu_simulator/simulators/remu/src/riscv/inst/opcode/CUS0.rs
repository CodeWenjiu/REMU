//! Custom opcode **CUS0** (`0b0001011` / `0x0B`, RV custom-0).
//!
//! - `func3=000` — **NN_LOAD_ACT**: **R-type**, `funct7=0`, `opcode=CUSTOM_0`. **`rs1`** = bias (index /
//!   address in GPR), **`rs2`** = value to write; **`rd`** reserved (often `x0`). No memory access.
//! - `func3=001` — **CTL** family: no GPR sources; remaining bits select opcode. **`NN_START`**: all
//!   non-opcode / non-funct3 bits zero (`0x100B`).
//! - `func3=010` — **NN_LOAD** family: I-type with **`rd` as destination** and **`rs1`** = which
//!   output to extract (**bias** / logit index, typically `0..9` in a GPR). **`NN_LOAD`**: `imm_i==0`;
//!   immediate field is `0`; **`rs1`** is not required to be `x0` (use `x0` if bias is literal `0`).
//!   Other `imm` variants reserved for future use.

use remu_state::StateError;

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
    /// `func3=000`, `funct7=0` — R-type: `rs1` = bias, `rs2` = value.
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

/// `func3=010` — `rd` present; **`rs1`** carries extract bias (which scalar to read).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NnLoadRdInst {
    /// I-type: `imm_i=0`, `simm12=0`; **`rd`** = destination, **`rs1`** = bias / output index.
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
        unsafe { core::hint::unreachable_unchecked() }
    };
    let _ = state;
    match op {
        Cus0Inst::NnLoadAct => {
            todo!()
        }
        Cus0Inst::Ctl(CtlInst::NnStart) => {
            todo!()
        }
        Cus0Inst::NnLoadRd(NnLoadRdInst::NnLoad) => {
            todo!()
        }
    }
}
