use super::{ImmType, RV32I, RV32M, Zicsr, Priv, RV32_I_PATTERN_ITER, RV32_M_PATTERN_ITER, RV_ZICSR_PATTERN_ITER, RV_PRIV_PATTERN_ITER};

pub fn rv32_i_decode(inst: u32) -> Option<(RV32I, ImmType)> {
    for (opcode, imm_type, (mask, value)) in RV32_I_PATTERN_ITER {
        if (inst & mask) == *value {
            return Some((opcode.clone(), imm_type.clone()));
        }
    }

    None
}

pub fn rv32_m_decode(inst: u32) -> Option<(RV32M, ImmType)> {
    for (opcode, imm_type, (mask, value)) in RV32_M_PATTERN_ITER {
        if (inst & mask) == *value {
            return Some((opcode.clone(), imm_type.clone()));
        }
    }

    None
}

pub fn rv_zicsr_decode(inst: u32) -> Option<(Zicsr, ImmType)> {
    for (opcode, imm_type, (mask, value)) in RV_ZICSR_PATTERN_ITER {
        if (inst & mask) == *value {
            return Some((opcode.clone(), imm_type.clone()));
        }
    }

    None
}

pub fn rv_priv_decode(inst: u32) -> Option<(Priv, ImmType)> {
    for (opcode, imm_type, (mask, value)) in RV_PRIV_PATTERN_ITER {
        if (inst & mask) == *value {
            return Some((opcode.clone(), imm_type.clone()));
        }
    }

    None
}
