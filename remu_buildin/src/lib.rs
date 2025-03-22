use remu_utils::ISA;

remu_macro::mod_pub!(riscv);

pub fn get_reset_vector(isa: ISA) -> u32 {
    match isa {
        ISA::RV32E => riscv::RESET_VECTOR,
        ISA::RV32I => riscv::RESET_VECTOR,
        ISA::RV32IM => riscv::RESET_VECTOR,
    }
}

pub fn get_buildin_img(isa: ISA) -> Vec<u8> {
    let img = match isa {
        ISA::RV32E => riscv::IMG.to_vec(),
        ISA::RV32I => riscv::IMG.to_vec(),
        ISA::RV32IM => riscv::IMG.to_vec(),
    };
    
    img.into_iter().flat_map(|u| u.to_le_bytes()).collect()
}

pub const READLINE_HISTORY_LENGTH: usize = 100;
