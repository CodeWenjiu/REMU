//! FFI bindings: C ABI types and functions matching difftest_abi.h
//! Spike owns its own data; no remu pointers held

use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uint};

/// Layout matches difftest_regs_t
#[repr(C)]
pub struct DifftestRegs {
    pub pc: u32,
    pub gpr: [u32; 32],
}

/// Memory layout: base + size only; Spike owns the memory
#[repr(C)]
pub struct DifftestMemLayout {
    pub guest_base: usize,
    pub size: usize,
}

/// Opaque context pointer
pub type SpikeDifftestCtx = *mut c_void;

#[allow(unsafe_code)]
unsafe extern "C" {
    pub fn spike_difftest_init(
        layout: *const DifftestMemLayout,
        n_regions: usize,
        init_pc: u32,
        init_gpr: *const u32,
        xlen: c_uint,
        isa: *const c_char,
    ) -> SpikeDifftestCtx;

    pub fn spike_difftest_copy_mem(
        ctx: SpikeDifftestCtx,
        guest_base: usize,
        data: *const u8,
        len: usize,
    );

    pub fn spike_difftest_sync_mem(
        ctx: SpikeDifftestCtx,
        guest_base: usize,
        data: *const u8,
        len: usize,
    );

    pub fn spike_difftest_read_mem(
        ctx: SpikeDifftestCtx,
        addr: usize,
        buf: *mut u8,
        len: usize,
    ) -> c_int;

    pub fn spike_difftest_write_mem(
        ctx: SpikeDifftestCtx,
        addr: usize,
        data: *const u8,
        len: usize,
    ) -> c_int;

    /// Returns 0 success, 1 program exit, -1 error
    pub fn spike_difftest_step(ctx: SpikeDifftestCtx) -> c_int;

    /// Pointer to Spike internal PC; for rv32 use as *const u32. Valid until next step/sync.
    pub fn spike_difftest_get_pc_ptr(ctx: SpikeDifftestCtx) -> *const u32;

    /// Pointer to Spike internal GPR; reg_t layout, gpr[i] at ptr[2*i] for rv32.
    pub fn spike_difftest_get_gpr_ptr(ctx: SpikeDifftestCtx) -> *const u32;

    /// Read one CSR by address (e.g. 0x300). Returns low 32 bits; 0 if not present.
    pub fn spike_difftest_get_csr(ctx: SpikeDifftestCtx, csr_addr: u16) -> u32;

    pub fn spike_difftest_sync_regs_to_spike(ctx: SpikeDifftestCtx, regs: *const DifftestRegs);

    pub fn spike_difftest_fini(ctx: SpikeDifftestCtx);
}
