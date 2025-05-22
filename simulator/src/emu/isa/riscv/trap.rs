pub enum Trap {
    Ebreak {is_trap_good: bool}, // for ysyx runtime
    Ecall {cause: u32},

    None,
}
