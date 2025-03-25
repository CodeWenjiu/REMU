#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!("../../ffi/bindings.rs"));

#[allow(dead_code)]
const DIFFTEST_TO_DUT: bool = false;
#[allow(dead_code)]
const DIFFTEST_TO_REF: bool = true;

#[test]
fn difftest_ffi_test() {
    unsafe {
        let mut regfile: riscv32_CPU_state = riscv32_CPU_state { gpr: [0; 32], pc: 0x80000000 };

        println!("{:?}", regfile);
        difftest_init(0);
        difftest_memcpy(0, std::ptr::null_mut(), 0, DIFFTEST_TO_REF);
        difftest_regcpy(&mut regfile as *mut _ as *mut std::os::raw::c_void, DIFFTEST_TO_DUT);
        println!("{:?}", regfile);
        difftest_exec(0);
        difftest_raise_intr(0);
    }
}