//! FFI bindings: C ABI types and function pointers loaded from libspike.so at runtime.
//! Spike owns its own data; no remu pointers held.

use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uint};

use libloading::Library;

/// Layout matches difftest_regs_t
#[repr(C)]
pub(crate) struct DifftestRegs {
    pub(crate) pc: u32,
    pub(crate) gpr: [u32; 32],
}

/// Memory layout: base + size only; Spike owns the memory
#[repr(C)]
pub(crate) struct DifftestMemLayout {
    pub(crate) guest_base: usize,
    pub(crate) size: usize,
}

/// Opaque context pointer
pub(crate) type SpikeDifftestCtx = *mut c_void;

// ---------------------------------------------------------------------------
// Function pointer table — loaded once via libloading, stored in a static.
// ---------------------------------------------------------------------------

#[allow(non_camel_case_types, unused)]
pub(crate) struct SpikeFns {
    pub init: unsafe extern "C" fn(
        layout: *const DifftestMemLayout,
        n_regions: usize,
        init_pc: u32,
        init_gpr: *const u32,
        xlen: c_uint,
        isa: *const c_char,
    ) -> SpikeDifftestCtx,

    pub copy_mem:
        unsafe extern "C" fn(ctx: SpikeDifftestCtx, guest_base: usize, data: *const u8, len: usize),

    pub read_mem:
        unsafe extern "C" fn(ctx: SpikeDifftestCtx, addr: usize, buf: *mut u8, len: usize) -> c_int,

    pub write_mem: unsafe extern "C" fn(
        ctx: SpikeDifftestCtx,
        addr: usize,
        data: *const u8,
        len: usize,
    ) -> c_int,

    /// Returns 0 success, 1 program exit, -1 error
    pub step: unsafe extern "C" fn(ctx: SpikeDifftestCtx) -> c_int,

    pub get_pc_ptr: unsafe extern "C" fn(ctx: SpikeDifftestCtx) -> *const u32,

    pub get_gpr_ptr: unsafe extern "C" fn(ctx: SpikeDifftestCtx) -> *const u32,

    pub get_csr: unsafe extern "C" fn(ctx: SpikeDifftestCtx, csr_addr: u16) -> u32,

    pub get_fpr: unsafe extern "C" fn(ctx: SpikeDifftestCtx, index: usize) -> u32,

    pub sync_regs_to_spike: unsafe extern "C" fn(ctx: SpikeDifftestCtx, regs: *const DifftestRegs),

    pub get_vlenb: unsafe extern "C" fn(ctx: SpikeDifftestCtx) -> usize,

    pub get_vr_ptr: unsafe extern "C" fn(ctx: SpikeDifftestCtx) -> *const u8,

    pub sync_vr_to_spike: unsafe extern "C" fn(ctx: SpikeDifftestCtx, data: *const u8, len: usize),

    pub write_vr_reg:
        unsafe extern "C" fn(ctx: SpikeDifftestCtx, index: usize, data: *const u8, len: usize),

    pub fini: unsafe extern "C" fn(ctx: SpikeDifftestCtx),
}

impl SpikeFns {
    pub(crate) fn from_library(lib: Library) -> Result<SpikeFns, String> {
        macro_rules! load {
            ($lib:ident, $name:literal) => {
                *$lib
                    .get::<unsafe extern "C" fn()>($name.as_bytes())
                    .map_err(|e| format!("symbol {}: {e}", $name))?
            };
        }

        // Safety: the loaded `.so` is never unloaded (held in static OnceLock).
        // Function pointers are copied out via deref and remain valid.
        Ok(unsafe {
            SpikeFns {
                init: std::mem::transmute(load!(lib, "spike_difftest_init")),
                copy_mem: std::mem::transmute(load!(lib, "spike_difftest_copy_mem")),
                read_mem: std::mem::transmute(load!(lib, "spike_difftest_read_mem")),
                write_mem: std::mem::transmute(load!(lib, "spike_difftest_write_mem")),
                step: std::mem::transmute(load!(lib, "spike_difftest_step")),
                get_pc_ptr: std::mem::transmute(load!(lib, "spike_difftest_get_pc_ptr")),
                get_gpr_ptr: std::mem::transmute(load!(lib, "spike_difftest_get_gpr_ptr")),
                get_csr: std::mem::transmute(load!(lib, "spike_difftest_get_csr")),
                get_fpr: std::mem::transmute(load!(lib, "spike_difftest_get_fpr")),
                sync_regs_to_spike: std::mem::transmute(load!(
                    lib,
                    "spike_difftest_sync_regs_to_spike"
                )),
                get_vlenb: std::mem::transmute(load!(lib, "spike_difftest_get_vlenb")),
                get_vr_ptr: std::mem::transmute(load!(lib, "spike_difftest_get_vr_ptr")),
                sync_vr_to_spike: std::mem::transmute(load!(
                    lib,
                    "spike_difftest_sync_vr_to_spike"
                )),
                write_vr_reg: std::mem::transmute(load!(lib, "spike_difftest_write_vr_reg")),
                fini: std::mem::transmute(load!(lib, "spike_difftest_fini")),
            }
        })
    }
}
