//! FFI bindings: C ABI types and function pointers loaded from libspike.so at runtime.
//! Spike owns its own data; no remu pointers held.

use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uint};

use dlopen2::wrapper::WrapperApi;

/// Layout matches difftest_regs_t (XLEN-wide values; RV32 keeps low 32 bits)
#[repr(C)]
pub(crate) struct DifftestRegs {
    pub(crate) pc: u64,
    pub(crate) gpr: [u64; 32],
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
// Function table: the field (method) names are the short names used by the
// simulator; `#[dlopen2_name]` maps them to the exported C symbols. Loading
// happens once in `Container::load` (see runtime.rs).
// ---------------------------------------------------------------------------

#[derive(WrapperApi)]
pub(crate) struct SpikeApi {
    #[dlopen2_name = "spike_difftest_init"]
    init: unsafe extern "C" fn(
        layout: *const DifftestMemLayout,
        n_regions: usize,
        init_pc: u64,
        init_gpr: *const u64,
        xlen: c_uint,
        isa: *const c_char,
    ) -> SpikeDifftestCtx,

    #[dlopen2_name = "spike_difftest_copy_mem"]
    copy_mem:
        unsafe extern "C" fn(ctx: SpikeDifftestCtx, guest_base: usize, data: *const u8, len: usize),

    #[dlopen2_name = "spike_difftest_read_mem"]
    read_mem:
        unsafe extern "C" fn(ctx: SpikeDifftestCtx, addr: usize, buf: *mut u8, len: usize) -> c_int,

    #[dlopen2_name = "spike_difftest_write_mem"]
    write_mem: unsafe extern "C" fn(
        ctx: SpikeDifftestCtx,
        addr: usize,
        data: *const u8,
        len: usize,
    ) -> c_int,

    /// Returns 0 success, 1 program exit, -1 error
    #[dlopen2_name = "spike_difftest_step"]
    step: unsafe extern "C" fn(ctx: SpikeDifftestCtx) -> c_int,

    #[dlopen2_name = "spike_difftest_get_pc_ptr"]
    get_pc_ptr: unsafe extern "C" fn(ctx: SpikeDifftestCtx) -> *const u64,

    #[dlopen2_name = "spike_difftest_get_gpr_ptr"]
    get_gpr_ptr: unsafe extern "C" fn(ctx: SpikeDifftestCtx) -> *const u64,

    #[dlopen2_name = "spike_difftest_get_csr"]
    get_csr: unsafe extern "C" fn(ctx: SpikeDifftestCtx, csr_addr: u16) -> u32,

    #[dlopen2_name = "spike_difftest_get_fpr"]
    get_fpr: unsafe extern "C" fn(ctx: SpikeDifftestCtx, index: usize) -> u32,

    #[dlopen2_name = "spike_difftest_sync_regs_to_spike"]
    sync_regs_to_spike: unsafe extern "C" fn(ctx: SpikeDifftestCtx, regs: *const DifftestRegs),

    #[dlopen2_name = "spike_difftest_get_vlenb"]
    get_vlenb: unsafe extern "C" fn(ctx: SpikeDifftestCtx) -> usize,

    #[dlopen2_name = "spike_difftest_get_vr_ptr"]
    get_vr_ptr: unsafe extern "C" fn(ctx: SpikeDifftestCtx) -> *const u8,

    #[dlopen2_name = "spike_difftest_sync_vr_to_spike"]
    sync_vr_to_spike: unsafe extern "C" fn(ctx: SpikeDifftestCtx, data: *const u8, len: usize),

    #[dlopen2_name = "spike_difftest_write_vr_reg"]
    write_vr_reg:
        unsafe extern "C" fn(ctx: SpikeDifftestCtx, index: usize, data: *const u8, len: usize),

    #[dlopen2_name = "spike_difftest_fini"]
    fini: unsafe extern "C" fn(ctx: SpikeDifftestCtx),
}
