//! RISC-V SYSTEM opcode: CSR read/write (CSRRW, CSRRS, CSRRC, CSRRWI, CSRRSI, CSRRCI).

use std::marker::PhantomData;

use remu_types::isa::reg::{Csr as CsrKind, RegAccess};

use crate::riscv::inst::{DecodedInst, csr, funct3, rd, rs1};

pub(crate) const OPCODE: u32 = 0b111_0011;

pub(crate) const INSTRUCTION_MIX: u32 = 20;

mod func3 {
    pub const CSRRW: u32 = 0b001;
    pub const CSRRS: u32 = 0b010;
    pub const CSRRC: u32 = 0b011;
    pub const CSRRWI: u32 = 0b101;
    pub const CSRRSI: u32 = 0b110;
    pub const CSRRCI: u32 = 0b111;
}

#[inline(always)]
fn do_csr<P: remu_state::StatePolicy>(
    state: &mut remu_state::State<P>,
    inst: &crate::riscv::inst::DecodedInst<P>,
    old_val: u32,
    new_val: u32,
) -> Result<(), remu_state::StateError> {
    let csr_kind = CsrKind::from_repr((inst.imm & 0xFFF) as u16).unwrap();
    state.reg.csr.write(csr_kind, new_val);
    state.reg.gpr.raw_write(inst.rd.into(), old_val);
    *state.reg.pc = state.reg.pc.wrapping_add(4);
    Ok(())
}

handler!(csrrw, state, inst, {
    let k = CsrKind::from_repr((inst.imm & 0xFFF) as u16).unwrap();
    let old = state.reg.csr.read(k);
    let new_val = state.reg.gpr.raw_read(inst.rs1.into());
    do_csr(state, inst, old, new_val)
});

handler!(csrrs, state, inst, {
    let k = CsrKind::from_repr((inst.imm & 0xFFF) as u16).unwrap();
    let old = state.reg.csr.read(k);
    let new_val = old | state.reg.gpr.raw_read(inst.rs1.into());
    do_csr(state, inst, old, new_val)
});

handler!(csrrc, state, inst, {
    let k = CsrKind::from_repr((inst.imm & 0xFFF) as u16).unwrap();
    let old = state.reg.csr.read(k);
    let new_val = old & !state.reg.gpr.raw_read(inst.rs1.into());
    do_csr(state, inst, old, new_val)
});

handler!(csrrwi, state, inst, {
    let k = CsrKind::from_repr((inst.imm & 0xFFF) as u16).unwrap();
    let old = state.reg.csr.read(k);
    let zimm = inst.rs1 as u32;
    do_csr(state, inst, old, zimm)
});

handler!(csrrsi, state, inst, {
    let k = CsrKind::from_repr((inst.imm & 0xFFF) as u16).unwrap();
    let old = state.reg.csr.read(k);
    let new_val = old | (inst.rs1 as u32);
    do_csr(state, inst, old, new_val)
});

handler!(csrrci, state, inst, {
    let k = CsrKind::from_repr((inst.imm & 0xFFF) as u16).unwrap();
    let old = state.reg.csr.read(k);
    let new_val = old & !(inst.rs1 as u32);
    do_csr(state, inst, old, new_val)
});

define_decode!(inst, {
    let f3 = funct3(inst);
    let csr_addr = csr(inst);
    let rd = rd(inst);
    let rs1 = rs1(inst);
    // Store 12-bit CSR address in imm for handlers
    let imm = csr_addr;

    match f3 {
        func3::CSRRW => DecodedInst::<P> {
            rd,
            rs1,
            rs2: 0,
            imm,
            handler: csrrw::<P>,
            _marker: PhantomData,
        },
        func3::CSRRS => DecodedInst::<P> {
            rd,
            rs1,
            rs2: 0,
            imm,
            handler: csrrs::<P>,
            _marker: PhantomData,
        },
        func3::CSRRC => DecodedInst::<P> {
            rd,
            rs1,
            rs2: 0,
            imm,
            handler: csrrc::<P>,
            _marker: PhantomData,
        },
        func3::CSRRWI => DecodedInst::<P> {
            rd,
            rs1,
            rs2: 0,
            imm,
            handler: csrrwi::<P>,
            _marker: PhantomData,
        },
        func3::CSRRSI => DecodedInst::<P> {
            rd,
            rs1,
            rs2: 0,
            imm,
            handler: csrrsi::<P>,
            _marker: PhantomData,
        },
        func3::CSRRCI => DecodedInst::<P> {
            rd,
            rs1,
            rs2: 0,
            imm,
            handler: csrrci::<P>,
            _marker: PhantomData,
        },
        _ => DecodedInst::<P>::default(),
    }
});
