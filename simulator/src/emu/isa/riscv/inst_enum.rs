#[derive(Debug, PartialEq, Clone)]
pub enum ImmType {
    I,
    S,
    B,
    U,
    J,
    R,
    N,
}

#[derive(Debug, PartialEq, Clone)]
pub enum RV32I {
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

    Lb,
    Lh,
    Lw,
    Lbu,
    Lhu,

    Sb,
    Sh,
    Sw,

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

#[derive(Debug, PartialEq, Clone)]
pub enum RV32M {
    Mul,

    Mulh,
    Mulhsu,
    Mulhu,

    Div,
    Divu,

    Rem,
    Remu,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Zicsr {
    Csrrw,
    Csrrs,
    Csrrc,

    Csrrwi,
    Csrrsi,
    Csrrci,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Priv {
    Mret,
}

#[derive(Debug, PartialEq, Clone)]
pub enum RISCV {
    RV32I(RV32I),
    RV32M(RV32M),
    Zicsr(Zicsr),
    Priv(Priv),
}
