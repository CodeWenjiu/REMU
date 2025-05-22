#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub enum ImmType {
    #[default]
    I,
    S,
    B,
    U,
    J,
    R,
    N,
}

#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub enum RV32IAL {
    #[default]
    Lui,
    Auipc,

    Jal,
    Jalr,
    
    Beq,
    Bne,
    Blt,
    Bge,
    Bltu,
    Bgeu,

    Addi,

    Slti,
    Sltiu,

    Xori,
    Ori,
    Andi,

    Slli,
    Srli,
    Srai,

    Add,
    Sub,

    Xor,
    Or,
    And,

    Slt,
    Sltu,

    Sll,
    Srl,
    Sra,

    Fence,
    Ecall,
    Ebreak,
}

#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub enum RV32ILS {
    #[default]
    Lb,
    Lh,
    Lw,
    Lbu,
    Lhu,

    Sb,
    Sh,
    Sw,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum RV32I {
    AL(RV32IAL),
    LS(RV32ILS),
}

impl Default for RV32I {
    fn default() -> Self {
        RV32I::AL(RV32IAL::default())
    }
}

#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub enum RV32M {
    #[default]
    Mul,

    Mulh,
    Mulhsu,
    Mulhu,

    Div,
    Divu,

    Rem,
    Remu,
}

#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub enum Zicsr {
    #[default]
    Csrrw,
    Csrrs,
    Csrrc,

    Csrrwi,
    Csrrsi,
    Csrrci,
}

#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub enum Priv {
    #[default]
    Mret,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum RISCV {
    RV32I(RV32I),
    RV32E(RV32I),
    RV32M(RV32M),
    Zicsr(Zicsr),
    Priv(Priv),
}

impl Default for RISCV {
    fn default() -> Self {
        RISCV::RV32I(RV32I::default())
    }
}
