use remu_utils::ISA;

remu_macro::mod_pub!(riscv);

pub fn get_buildin_img(isa: ISA) -> &'static [u32] {
    match isa {
        ISA::RV32E => riscv::IMG,
        ISA::RV32I => riscv::IMG,
        ISA::RV32IM => riscv::IMG,
    }
}
