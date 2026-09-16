#![allow(non_snake_case)]

use remu_isa::Xlen;
use remu_isa::isa::RvIsa;
use remu_isa::isa::extension_v::VExtensionConfig;
use remu_state::{StateError, StatePolicy};

remu_macro::mod_pub!(crate, opcode);
remu_macro::mod_prv!(bytes);

pub(crate) use bytes::{
    csr, funct3, funct7, imm_b, imm_i, imm_j, imm_s, imm_u, opcode, rd, rs1, rs2,
};

use crate::riscv::opcode::{
    AUIPC, BRANCH, CUS0, JAL, JALR, LOAD, LOAD_FP, LUI, MISC_MEM, OP, OP_IMM, OP_V, STORE,
    STORE_FP, SYSTEM, UNKNOWN,
};

/// Instruction kind: variant per concrete instruction (sub-enums flattened so
/// the execute dispatch is a single jump table, not nested lookups).
/// Opcode-specific data stays payload-bearing (e.g. `V(VInst)`).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) enum Inst {
    Lui,
    Auipc,
    Jal,
    Jalr,
    // Branch (flat)
    Beq,
    Bne,
    Blt,
    Bge,
    Bltu,
    Bgeu,
    // OP_IMM (flat)
    Addi,
    Slli,
    Slti,
    Sltiu,
    Xori,
    Srli,
    Srai,
    Ori,
    Andi,
    // OP (flat)
    Add,
    Sub,
    Sll,
    Slt,
    Sltu,
    Xor,
    Srl,
    Sra,
    Or,
    And,
    Mul,
    Mulh,
    Mulhsu,
    Mulhu,
    Div,
    Divu,
    Rem,
    Remu,
    // LOAD (flat)
    Lb,
    Lh,
    Lw,
    Ld,
    Lbu,
    Lhu,
    Lwu,
    // STORE (flat)
    Sb,
    Sh,
    Sw,
    Sd,
    LoadFp(LOAD_FP::LoadFpInst),
    StoreFp(STORE_FP::StoreFpInst),
    MiscMem(MISC_MEM::MiscMemInst),
    System(SYSTEM::SystemInst),
    V(OP_V::VInst),
    Cus0(CUS0::Cus0Inst),
    // RV64 word ops (opcodes 0x1b / 0x3b)
    Addiw,
    Slliw,
    Srliw,
    Sraiw,
    Addw,
    Subw,
    Sllw,
    Srlw,
    Sraw,
    Mulw,
    Divw,
    Divuw,
    Remw,
    Remuw,
    #[default]
    Unknown,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct DecodedInst {
    pub(crate) rs1: u8,
    pub(crate) rs2: u8,
    pub(crate) rd: u8,
    pub imm: u32,
    pub(crate) inst: Inst,
}

/// Compile-time XLEN test. `XLEN::BITS` is an associated const, so every
/// `if rv64::<P>()` folds to a constant per instantiation and LLVM drops the
/// RV64-only branch (and its callees) from RV32 monomorphizations entirely.
#[inline(always)]
pub(crate) const fn rv64<P: StatePolicy>() -> bool {
    <<P::ISA as RvIsa>::XLEN as Xlen>::BITS == 64
}

#[inline(always)]
pub(crate) fn decode<P: StatePolicy>(inst: u32) -> DecodedInst {
    let op = opcode(inst);
    match op {
        LUI::OPCODE => LUI::decode::<P>(inst),
        AUIPC::OPCODE => AUIPC::decode::<P>(inst),
        JAL::OPCODE => JAL::decode::<P>(inst),
        JALR::OPCODE => JALR::decode::<P>(inst),
        BRANCH::OPCODE => BRANCH::decode::<P>(inst),
        LOAD::OPCODE => LOAD::decode::<P>(inst),
        LOAD_FP::OPCODE => LOAD_FP::decode::<P>(inst),
        STORE::OPCODE => STORE::decode::<P>(inst),
        STORE_FP::OPCODE => STORE_FP::decode::<P>(inst),
        OP_IMM::OPCODE => OP_IMM::decode::<P>(inst),
        OP_IMM::OPCODE_W => {
            // RV64-only word-immediate ops: illegal on RV32.
            if rv64::<P>() {
                OP_IMM::decode_w::<P>(inst)
            } else {
                UNKNOWN::decode::<P>(inst)
            }
        }
        OP::OPCODE => OP::decode::<P>(inst),
        OP::OPCODE_W => {
            // RV64-only word ALU ops: illegal on RV32.
            if rv64::<P>() {
                OP::decode_w::<P>(inst)
            } else {
                UNKNOWN::decode::<P>(inst)
            }
        }
        MISC_MEM::OPCODE => MISC_MEM::decode::<P>(inst),
        SYSTEM::OPCODE => SYSTEM::decode::<P>(inst),
        OP_V::OPCODE => {
            if <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB > 0 {
                OP_V::decode::<P>(inst)
            } else {
                UNKNOWN::decode::<P>(inst)
            }
        }
        CUS0::OPCODE => {
            if <P::ISA as RvIsa>::HAS_WJ_CUS0 {
                CUS0::decode::<P>(inst)
            } else {
                UNKNOWN::decode::<P>(inst)
            }
        }
        _ => UNKNOWN::decode::<P>(inst),
    }
}

#[inline(always)]
pub(crate) fn execute<P: StatePolicy, C: crate::ExecuteContext<P>>(
    ctx: &mut C,
    decoded: &DecodedInst,
    pc: <P::ISA as RvIsa>::XLEN,
) -> Result<<P::ISA as RvIsa>::XLEN, StateError> {
    match decoded.inst {
        Inst::Lui => LUI::execute(ctx, decoded, pc),
        Inst::Auipc => AUIPC::execute(ctx, decoded, pc),
        Inst::Jal => JAL::execute(ctx, decoded, pc),
        Inst::Jalr => JALR::execute(ctx, decoded, pc),
        // Flags flattened into a single dispatch table (no second jump table).
        Inst::Beq => BRANCH::execute_beq(ctx, decoded, pc),
        Inst::Bne => BRANCH::execute_bne(ctx, decoded, pc),
        Inst::Blt => BRANCH::execute_blt(ctx, decoded, pc),
        Inst::Bge => BRANCH::execute_bge(ctx, decoded, pc),
        Inst::Bltu => BRANCH::execute_bltu(ctx, decoded, pc),
        Inst::Bgeu => BRANCH::execute_bgeu(ctx, decoded, pc),
        Inst::Addi => OP_IMM::execute_addi(ctx, decoded, pc),
        Inst::Slli => OP_IMM::execute_slli(ctx, decoded, pc),
        Inst::Slti => OP_IMM::execute_slti(ctx, decoded, pc),
        Inst::Sltiu => OP_IMM::execute_sltiu(ctx, decoded, pc),
        Inst::Xori => OP_IMM::execute_xori(ctx, decoded, pc),
        Inst::Srli => OP_IMM::execute_srli(ctx, decoded, pc),
        Inst::Srai => OP_IMM::execute_srai(ctx, decoded, pc),
        Inst::Ori => OP_IMM::execute_ori(ctx, decoded, pc),
        Inst::Andi => OP_IMM::execute_andi(ctx, decoded, pc),
        Inst::Add => OP::execute_add(ctx, decoded, pc),
        Inst::Sub => OP::execute_sub(ctx, decoded, pc),
        Inst::Sll => OP::execute_sll(ctx, decoded, pc),
        Inst::Slt => OP::execute_slt(ctx, decoded, pc),
        Inst::Sltu => OP::execute_sltu(ctx, decoded, pc),
        Inst::Xor => OP::execute_xor(ctx, decoded, pc),
        Inst::Srl => OP::execute_srl(ctx, decoded, pc),
        Inst::Sra => OP::execute_sra(ctx, decoded, pc),
        Inst::Or => OP::execute_or(ctx, decoded, pc),
        Inst::And => OP::execute_and(ctx, decoded, pc),
        Inst::Mul => OP::execute_mul(ctx, decoded, pc),
        Inst::Mulh => OP::execute_mulh(ctx, decoded, pc),
        Inst::Mulhsu => OP::execute_mulhsu(ctx, decoded, pc),
        Inst::Mulhu => OP::execute_mulhu(ctx, decoded, pc),
        Inst::Div => OP::execute_div(ctx, decoded, pc),
        Inst::Divu => OP::execute_divu(ctx, decoded, pc),
        Inst::Rem => OP::execute_rem(ctx, decoded, pc),
        Inst::Remu => OP::execute_remu(ctx, decoded, pc),
        Inst::Lb => LOAD::execute_lb(ctx, decoded, pc),
        Inst::Lh => LOAD::execute_lh(ctx, decoded, pc),
        Inst::Lw => LOAD::execute_lw(ctx, decoded, pc),
        // RV64-only widths: the guard folds to false for RV32 monomorphizations,
        // so these arms (and their inlined execute bodies) are never compiled.
        Inst::Ld if rv64::<P>() => LOAD::execute_ld(ctx, decoded, pc),
        Inst::Lbu => LOAD::execute_lbu(ctx, decoded, pc),
        Inst::Lhu => LOAD::execute_lhu(ctx, decoded, pc),
        Inst::Lwu if rv64::<P>() => LOAD::execute_lwu(ctx, decoded, pc),
        Inst::Sb => STORE::execute_sb(ctx, decoded, pc),
        Inst::Sh => STORE::execute_sh(ctx, decoded, pc),
        Inst::Sw => STORE::execute_sw(ctx, decoded, pc),
        Inst::Sd if rv64::<P>() => STORE::execute_sd(ctx, decoded, pc),
        Inst::LoadFp(..) => LOAD_FP::execute(ctx, decoded, pc),
        Inst::StoreFp(..) => STORE_FP::execute(ctx, decoded, pc),
        Inst::MiscMem(..) => MISC_MEM::execute(ctx, decoded, pc),
        Inst::System(..) => SYSTEM::execute(ctx, decoded, pc),
        Inst::V(..) => {
            if <<P::ISA as RvIsa>::VConfig as VExtensionConfig>::VLENB > 0 {
                OP_V::execute(ctx, decoded, pc)
            } else {
                unsafe { core::hint::unreachable_unchecked() }
            }
        }
        Inst::Cus0(..) => {
            if <P::ISA as RvIsa>::HAS_WJ_CUS0 {
                CUS0::execute(ctx, decoded, pc)
            } else {
                UNKNOWN::execute(ctx, decoded, pc)
            }
        }
        Inst::Addiw if rv64::<P>() => OP_IMM::execute_addiw(ctx, decoded, pc),
        Inst::Slliw if rv64::<P>() => OP_IMM::execute_slliw(ctx, decoded, pc),
        Inst::Srliw if rv64::<P>() => OP_IMM::execute_srliw(ctx, decoded, pc),
        Inst::Sraiw if rv64::<P>() => OP_IMM::execute_sraiw(ctx, decoded, pc),
        // RV64-only word-ALU ops; same folding rationale as the LOAD/STORE arms.
        Inst::Addw if rv64::<P>() => OP::execute_addw(ctx, decoded, pc),
        Inst::Subw if rv64::<P>() => OP::execute_subw(ctx, decoded, pc),
        Inst::Sllw if rv64::<P>() => OP::execute_sllw(ctx, decoded, pc),
        Inst::Srlw if rv64::<P>() => OP::execute_srlw(ctx, decoded, pc),
        Inst::Sraw if rv64::<P>() => OP::execute_sraw(ctx, decoded, pc),
        Inst::Mulw if rv64::<P>() => OP::execute_mulw(ctx, decoded, pc),
        Inst::Divw if rv64::<P>() => OP::execute_divw(ctx, decoded, pc),
        Inst::Divuw if rv64::<P>() => OP::execute_divuw(ctx, decoded, pc),
        Inst::Remw if rv64::<P>() => OP::execute_remw(ctx, decoded, pc),
        Inst::Remuw if rv64::<P>() => OP::execute_remuw(ctx, decoded, pc),
        Inst::Unknown => UNKNOWN::execute(ctx, decoded, pc),
        // Defensive tail: exhaustive by construction (every variant has an arm
        // above, gated by `rv64` where RV64-only). For RV32 monomorphizations
        // the gated arms fold away, leaving this arm unreachable and DCEd too.
        _ => UNKNOWN::execute(ctx, decoded, pc),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use remu_isa::isa::extension_enum::{RV32I, RV64IM};
    use remu_state::StateFastProfile;

    fn is_known<P: StatePolicy>(inst: u32) -> bool {
        core::mem::discriminant(&decode::<P>(inst).inst) != core::mem::discriminant(&Inst::Unknown)
    }

    #[test]
    fn rv32_rejects_rv64_only_encodings() {
        type P32 = StateFastProfile<RV32I>;
        // Word-immediate / word-ALU opcodes (0x1b / 0x3b) are RV64-only.
        assert!(!is_known::<P32>(0x0005051b)); // addiw a0, a0, 0
        assert!(!is_known::<P32>(0x02a7053b)); // mulw a0, a4, a0
        assert!(!is_known::<P32>(0x4075559b)); // sraiw a1, a0, 7
        // RV64-only load/store widths.
        assert!(!is_known::<P32>(0x00053503)); // ld a0, 0(a0)
        assert!(!is_known::<P32>(0x00056503)); // lwu a0, 0(a0)
        assert!(!is_known::<P32>(0x00a53023)); // sd a0, 0(a0)
        // Sanity: shared encodings stay legal.
        assert!(is_known::<P32>(0x00b50533)); // add a0, a0, a1
        assert!(is_known::<P32>(0x00052503)); // lw a0, 0(a0)
        assert!(is_known::<P32>(0x00a52023)); // sw a0, 0(a0)
    }

    #[test]
    fn rv64_accepts_rv64_encodings() {
        type P64 = StateFastProfile<RV64IM>;
        assert!(is_known::<P64>(0x0005051b)); // addiw
        assert!(is_known::<P64>(0x02a7053b)); // mulw
        assert!(is_known::<P64>(0x4075559b)); // sraiw
        assert!(is_known::<P64>(0x00053503)); // ld
        assert!(is_known::<P64>(0x00056503)); // lwu
        assert!(is_known::<P64>(0x00a53023)); // sd
    }
}
#[allow(dead_code)]

pub(crate) const RV32_INSTRUCTION_MIX: &[(u32, u32)] = &[
    (AUIPC::OPCODE, AUIPC::INSTRUCTION_MIX),
    (BRANCH::OPCODE, BRANCH::INSTRUCTION_MIX),
    (JAL::OPCODE, JAL::INSTRUCTION_MIX),
    (JALR::OPCODE, JALR::INSTRUCTION_MIX),
    (LOAD::OPCODE, LOAD::INSTRUCTION_MIX),
    (LOAD_FP::OPCODE, LOAD_FP::INSTRUCTION_MIX),
    (LUI::OPCODE, LUI::INSTRUCTION_MIX),
    (MISC_MEM::OPCODE, MISC_MEM::INSTRUCTION_MIX),
    (OP::OPCODE, OP::INSTRUCTION_MIX),
    (OP_IMM::OPCODE, OP_IMM::INSTRUCTION_MIX),
    (STORE::OPCODE, STORE::INSTRUCTION_MIX),
    (STORE_FP::OPCODE, STORE_FP::INSTRUCTION_MIX),
    (SYSTEM::OPCODE, SYSTEM::INSTRUCTION_MIX),
    (OP_V::OPCODE, OP_V::INSTRUCTION_MIX),
    (CUS0::OPCODE, CUS0::INSTRUCTION_MIX),
    (UNKNOWN::OPCODE, UNKNOWN::INSTRUCTION_MIX),
];
