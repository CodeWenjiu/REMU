//! RISC-V SYSTEM opcode: CSR read/write (CSRRW, CSRRS, CSRRC, CSRRWI, CSRRSI, CSRRCI).

use remu_types::isa::reg::{Csr as CsrKind, RegAccess};

use crate::riscv::inst::{csr, funct3, rd, rs1, DecodedInst, Inst};

pub(crate) const OPCODE: u32 = 0b111_0011;
pub(crate) const INSTRUCTION_MIX: u32 = 20;

mod func3 {
    pub(super) const CSRRW: u32 = 0b001;
    pub(super) const CSRRS: u32 = 0b010;
    pub(super) const CSRRC: u32 = 0b011;
    pub(super) const CSRRWI: u32 = 0b101;
    pub(super) const CSRRSI: u32 = 0b110;
    pub(super) const CSRRCI: u32 = 0b111;
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum SystemInst {
    Csrrw,
    Csrrs,
    Csrrc,
    Csrrwi,
    Csrrsi,
    Csrrci,
}

#[inline(always)]
pub(crate) fn decode<P: remu_state::StatePolicy>(inst: u32) -> DecodedInst {
    let f3 = funct3(inst);
    let sys = match f3 {
        func3::CSRRW => SystemInst::Csrrw,
        func3::CSRRS => SystemInst::Csrrs,
        func3::CSRRC => SystemInst::Csrrc,
        func3::CSRRWI => SystemInst::Csrrwi,
        func3::CSRRSI => SystemInst::Csrrsi,
        func3::CSRRCI => SystemInst::Csrrci,
        _ => return DecodedInst::default(),
    };
    let csr_addr = csr(inst);
    DecodedInst {
        rd: rd(inst),
        rs1: rs1(inst),
        rs2: 0,
        imm: csr_addr,
        inst: Inst::System(sys),
    }
}

#[inline(always)]
fn do_csr<P: remu_state::StatePolicy>(
    state: &mut remu_state::State<P>,
    decoded: &DecodedInst,
    old_val: u32,
    new_val: u32,
) -> Result<(), remu_state::StateError> {
    let csr_kind = CsrKind::from_repr((decoded.imm & 0xFFF) as u16).unwrap();
    state.reg.csr.write(csr_kind, new_val);
    state.reg.gpr.raw_write(decoded.rd.into(), old_val);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

#[inline(always)]
pub(crate) fn execute<P: remu_state::StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
) -> Result<(), remu_state::StateError> {
    let state = ctx.state_mut();
    let Inst::System(sys) = decoded.inst else { unreachable!() };
    let k = CsrKind::from_repr((decoded.imm & 0xFFF) as u16).unwrap();
    let old = state.reg.read_csr(k);
    let new_val = match sys {
        SystemInst::Csrrw => state.reg.gpr.raw_read(decoded.rs1.into()),
        SystemInst::Csrrs => old | state.reg.gpr.raw_read(decoded.rs1.into()),
        SystemInst::Csrrc => old & !state.reg.gpr.raw_read(decoded.rs1.into()),
        SystemInst::Csrrwi => decoded.rs1 as u32,
        SystemInst::Csrrsi => old | (decoded.rs1 as u32),
        SystemInst::Csrrci => old & !(decoded.rs1 as u32),
    };
    do_csr(state, decoded, old, new_val)
}
