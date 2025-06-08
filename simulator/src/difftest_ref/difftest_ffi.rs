#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::{ffi::c_int, os::raw::c_void};

use dlopen2::{wrapper::Container, wrapper::WrapperApi};
use logger::Logger;
use remu_macro::log_error;
use remu_utils::ProcessResult;
use state::reg::{AnyRegfile, RegfileIo};

use super::DifftestRefFfiApi;

#[allow(dead_code)]
const DIFFTEST_TO_DUT: bool = false;
#[allow(dead_code)]
const DIFFTEST_TO_REF: bool = true;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DifftestFFICpuState {
    pub gpr: [u32; 32usize],
    pub pc: u32,
}

#[derive(WrapperApi)]
struct DifftestFFIApi {
    difftest_init: unsafe extern "C" fn(port: c_int),
    difftest_memcpy: unsafe extern "C" fn(addr: u32, buf: *mut c_void, n: u64, direction: bool),
    difftest_regcpy: unsafe extern "C" fn(dut: *mut c_void, direction: bool),
    difftest_exec: unsafe extern "C" fn(n: u64),
    difftest_raise_intr: unsafe extern "C" fn(NO: u64),
}

impl From<&AnyRegfile> for DifftestFFICpuState {
    fn from(regfile: &AnyRegfile) -> Self {
        let mut gpr = [0; 32];
        for (i, a) in regfile.get_gprs().iter().enumerate() {
            gpr[i] = *a;
        }
        DifftestFFICpuState { gpr, pc: regfile.read_pc() }
    }
}

pub struct FFI {
    container: Container<DifftestFFIApi>
}

impl FFI {
    pub fn new(so_path: &str) -> Self {
        let container: Container<DifftestFFIApi> = unsafe { Container::load(so_path) }
            .expect("Could not open library or load symbols");
        FFI { container }
    }
}

impl DifftestRefFfiApi for FFI {
    fn init(&mut self, regfile: &AnyRegfile, bin: Vec<u8>, reset_vector: u32) {
        unsafe {
            (self.container.difftest_init)(0);

            let mut regfile = DifftestFFICpuState::from(regfile);
            (self.container.difftest_regcpy)(&mut regfile as *mut _ as *mut std::os::raw::c_void, DIFFTEST_TO_REF);

            (self.container.difftest_memcpy)(reset_vector, bin.as_ptr() as *mut std::os::raw::c_void, bin.len() as u64, DIFFTEST_TO_REF);
        }
    }

    fn step_cycle(&mut self) -> ProcessResult<()> {
        unsafe {
            (self.container.difftest_exec)(1);
        }
        Ok(())
    }

    fn test_reg(&self, dut: &AnyRegfile) -> ProcessResult<()> {
        unsafe {
            let mut regfile: DifftestFFICpuState = DifftestFFICpuState { gpr: [0; 32], pc: 0x80000000 };
            (self.container.difftest_regcpy)(&mut regfile as *mut _ as *mut std::os::raw::c_void, DIFFTEST_TO_DUT);
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
                (self.container.difftest_memcpy)(addr, buf.as_mut_ptr() as *mut std::os::raw::c_void, 4, DIFFTEST_TO_DUT);
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
            let mut regfile = DifftestFFICpuState::from(target);
            (self.container.difftest_regcpy)(&mut regfile as *mut _ as *mut std::os::raw::c_void, DIFFTEST_TO_REF);
        }
    }

    fn set_mem(&self, addr:u32, data:Vec<u8>) {
        unsafe {
            (self.container.difftest_memcpy)(addr, data.as_ptr() as *mut std::os::raw::c_void, data.len() as u64, DIFFTEST_TO_REF);
        }
    }
}
