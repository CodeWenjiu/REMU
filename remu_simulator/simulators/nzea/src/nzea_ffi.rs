//! Raw FFI to C++ glue (nzea_wrapper.cpp): create/destroy/drive the Verilated model.
//! Model key selects the Verilated model (`core|tile` + ISA).

use std::ffi::{c_char, c_void};

use remu_types::isa::extension_enum::{RV32I, RV32I_wjCus0, RV32IM, RV32IM_wjCus0};

unsafe extern "C" {
    pub(crate) fn nzea_create(model: *const c_char) -> *mut c_void;
    pub(crate) fn nzea_destroy(sim: *mut c_void, model: *const c_char);
    pub(crate) fn nzea_set_clock(sim: *mut c_void, model: *const c_char, val: i32);
    pub(crate) fn nzea_set_reset(sim: *mut c_void, model: *const c_char, val: i32);
    pub(crate) fn nzea_eval(sim: *mut c_void, model: *const c_char);
    pub(crate) fn nzea_trace_open(sim: *mut c_void, model: *const c_char, filename: *const c_char);
    pub(crate) fn nzea_trace_dump(sim: *mut c_void);
    #[allow(dead_code)]
    pub(crate) fn nzea_trace_close(sim: *mut c_void);
}

/// ISA string for nzea DPI; must match nzea `just dump --isa <str>`.
pub trait NzeaIsa: remu_types::isa::RvIsa {
    const NZEA_ISA_STR: &'static str;
}

impl NzeaIsa for RV32I {
    const NZEA_ISA_STR: &'static str = "riscv32i";
}

impl NzeaIsa for RV32IM {
    const NZEA_ISA_STR: &'static str = "riscv32im";
}

impl NzeaIsa for RV32I_wjCus0 {
    const NZEA_ISA_STR: &'static str = "riscv32i_wjCus0";
}

impl NzeaIsa for RV32IM_wjCus0 {
    const NZEA_ISA_STR: &'static str = "riscv32im_wjCus0";
}
