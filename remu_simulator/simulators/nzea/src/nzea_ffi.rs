//! FFI bindings: function pointers loaded from libnzea.so at runtime.
//! Each (platform, isa) combination loads its own .so.

use std::ffi::{c_char, c_void};

use libloading::Library;

use remu_isa::isa::extension_enum::{RV32I, RV32I_wjCus0, RV32IM, RV32IM_wjCus0};

/// Callback invoked by `nzea_iter_stats` for each stat_* counter.
pub(crate) type NzeaStatCallback =
    unsafe extern "C" fn(name: *const c_char, value: u32, userdata: *mut c_void);

// ---------------------------------------------------------------------------
// Function pointer table
// ---------------------------------------------------------------------------

#[allow(non_camel_case_types)]
pub(crate) struct NzeaFns {
    pub create: unsafe extern "C" fn(model: *const i8) -> *mut c_void,
    pub destroy: unsafe extern "C" fn(sim: *mut c_void, model: *const i8),
    pub set_clock: unsafe extern "C" fn(sim: *mut c_void, model: *const i8, val: i32),
    pub set_reset: unsafe extern "C" fn(sim: *mut c_void, model: *const i8, val: i32),
    pub eval: unsafe extern "C" fn(sim: *mut c_void, model: *const i8),
    pub trace_open: unsafe extern "C" fn(sim: *mut c_void, model: *const i8, filename: *const i8),
    pub trace_dump: unsafe extern "C" fn(sim: *mut c_void),
    pub iter_stats: unsafe extern "C" fn(
        sim: *mut c_void,
        cb: Option<NzeaStatCallback>,
        userdata: *mut c_void,
    ) -> i32,
}

impl NzeaFns {
    pub(crate) fn from_library(lib: Library) -> Result<NzeaFns, String> {
        macro_rules! load {
            ($lib:ident, $name:literal) => {
                *$lib
                    .get::<unsafe extern "C" fn()>($name.as_bytes())
                    .map_err(|e| format!("symbol {}: {e}", $name))?
            };
        }

        Ok(unsafe {
            NzeaFns {
                create: std::mem::transmute(load!(lib, "nzea_create")),
                destroy: std::mem::transmute(load!(lib, "nzea_destroy")),
                set_clock: std::mem::transmute(load!(lib, "nzea_set_clock")),
                set_reset: std::mem::transmute(load!(lib, "nzea_set_reset")),
                eval: std::mem::transmute(load!(lib, "nzea_eval")),
                trace_open: std::mem::transmute(load!(lib, "nzea_trace_open")),
                trace_dump: std::mem::transmute(load!(lib, "nzea_trace_dump")),
                iter_stats: std::mem::transmute(load!(lib, "nzea_iter_stats")),
            }
        })
    }
}

// ---------------------------------------------------------------------------
// ISA string mapping
// ---------------------------------------------------------------------------

/// ISA string for nzea DPI; must match nzea `just dump --isa <str>`.
pub trait NzeaIsa: remu_isa::isa::RvIsa {
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
