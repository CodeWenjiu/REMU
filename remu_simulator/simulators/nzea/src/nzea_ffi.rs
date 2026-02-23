//! Raw FFI to C++ glue (nzea_wrapper.cpp): create/destroy/drive the Verilated model.

use std::ffi::c_void;

unsafe extern "C" {
    pub(crate) fn nzea_create() -> *mut c_void;
    pub(crate) fn nzea_destroy(sim: *mut c_void);
    pub(crate) fn nzea_set_clock(sim: *mut c_void, val: i32);
    pub(crate) fn nzea_set_reset(sim: *mut c_void, val: i32);
    pub(crate) fn nzea_eval(sim: *mut c_void);
}
