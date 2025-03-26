#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use logger::Logger;
use remu_macro::log_error;
use remu_utils::ProcessResult;
use state::reg::{AnyRegfile, RegfileIo};

use super::DifftestRef;

include!(concat!("../../bindings.rs"));

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

pub struct Spike {
    
}

impl DifftestRef for Spike {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        unsafe {
            difftest_exec(1);
        }
        Ok(())
    }

    fn test_reg(&self, dut: AnyRegfile) -> bool {
        unsafe {
            let mut regfile: riscv32_CPU_state = riscv32_CPU_state { gpr: [0; 32], pc: 0x80000000 };
            difftest_regcpy(&mut regfile as *mut _ as *mut std::os::raw::c_void, DIFFTEST_TO_DUT);
            for (i, (a, b)) in regfile.gpr.iter().zip(dut.get_gprs().iter()).enumerate() {
                if a != b {
                    log_error!(format!(
                        "Dut GPR[{}]: {:#010x}, Ref GPR[{}]: {:#010x}",
                        i, a, i, b
                    ));
                    return false;
                }
            }
            return true;
        }
    }
}
