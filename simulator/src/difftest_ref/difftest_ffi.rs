#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use logger::Logger;
use remu_macro::log_error;
use remu_utils::ProcessResult;
use state::reg::{AnyRegfile, RegfileIo};

use super::DifftestRefApi;

include!(concat!("../../bindings.rs"));

#[allow(dead_code)]
const DIFFTEST_TO_DUT: bool = false;
#[allow(dead_code)]
const DIFFTEST_TO_REF: bool = true;

impl From<&AnyRegfile> for riscv32_CPU_state {
    fn from(regfile: &AnyRegfile) -> Self {
        let mut gpr = [0; 32];
        for (i, a) in regfile.get_gprs().iter().enumerate() {
            gpr[i] = *a;
        }
        riscv32_CPU_state { gpr, pc: regfile.read_pc() }
    }
}

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

pub fn difftestffi_init(regfile: &AnyRegfile, bin: Vec<u8>, reset_vector: u32) {
    unsafe {
        difftest_init(0);
        
        let mut regfile = riscv32_CPU_state::from(regfile);
        difftest_regcpy(&mut regfile as *mut _ as *mut std::os::raw::c_void, DIFFTEST_TO_REF);

        difftest_memcpy(reset_vector, bin.as_ptr() as *mut std::os::raw::c_void, bin.len() as u64, DIFFTEST_TO_REF);
    }
}

pub struct Spike {
}

impl DifftestRefApi for Spike {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        unsafe {
            difftest_exec(1);
        }
        Ok(())
    }

    fn test_reg(&self, dut: &AnyRegfile) -> ProcessResult<()> {
        unsafe {
            let mut regfile: riscv32_CPU_state = riscv32_CPU_state { gpr: [0; 32], pc: 0x80000000 };
            difftest_regcpy(&mut regfile as *mut _ as *mut std::os::raw::c_void, DIFFTEST_TO_DUT);
            if regfile.pc != dut.read_pc() {
                log_error!(format!(
                    "Dut PC: {:#010x}, Ref PC: {:#010x}",
                    dut.read_pc(),
                    regfile.pc
                ));
                return Err(remu_utils::ProcessError::Recoverable);
            }

            for (i, (a, b)) in regfile.gpr.iter().zip(dut.get_gprs().iter()).enumerate() {
                if a != b {
                    let name = dut.gpr_into_str(i as u32);
                    log_error!(format!(
                        "Dut {}: [{:#010x}], Ref {}: [{:#010x}]",
                        &name, b, &name, a
                    ));
                    return Err(remu_utils::ProcessError::Recoverable);
                }
            }

            Ok(())
        }
    }

    fn test_mem(&mut self,watchpoint:Vec<(u32,u32)>) -> ProcessResult<()> {
        for (addr, data) in watchpoint {
            unsafe {
                let mut buf = [0; 4];
                difftest_memcpy(addr, buf.as_mut_ptr() as *mut std::os::raw::c_void, 4, DIFFTEST_TO_DUT);
                let ref_data = u32::from_le_bytes(buf);
                if ref_data != data {
                    log_error!(format!(
                        "Dut Memory: {:#010x} : {:#010x}, Ref Memory: {:#010x} : {:#010x}",
                        addr, data,
                        addr, ref_data
                    ));
                    return Err(remu_utils::ProcessError::Recoverable);
                }
            }
        }
        Ok(())
    }

    fn set_ref(&self, target: &AnyRegfile) {
        unsafe {
            let mut regfile = riscv32_CPU_state::from(target);
            difftest_regcpy(&mut regfile as *mut _ as *mut std::os::raw::c_void, DIFFTEST_TO_REF);
        }
    }

    fn set_mem(&self, addr:u32, data:Vec<u8>) {
        unsafe {
            difftest_memcpy(addr, data.as_ptr() as *mut std::os::raw::c_void, data.len() as u64, DIFFTEST_TO_REF);
        }
    }
}
