use remu_utils::ISA;

remu_macro::mod_pub!(riscv);

pub fn get_reset_vector(isa: ISA) -> u32 {
    match isa {
        ISA::RV32E => riscv::RESET_VECTOR,
        ISA::RV32I => riscv::RESET_VECTOR,
        ISA::RV32IM => riscv::RESET_VECTOR,
    }
}

pub fn get_buildin_img(isa: ISA) -> &'static [u32] {
    match isa {
        ISA::RV32E => riscv::IMG,
        ISA::RV32I => riscv::IMG,
        ISA::RV32IM => riscv::IMG,
    }
}

pub const READLINE_HISTORY_LENGTH: usize = 100;
