//! CPU / CSR setup before application logic (vector `mstatus.VS`, etc.).

/// Run once at the beginning of `main` (or call from [`crate::heap::init`]).
///
/// When compiled with `target_feature=+zve32x`, sets `mstatus.VS` to `Initial` so
/// Zve instructions are legal (same role as riscv-rt’s FS init for float).
///
/// # Safety
///
/// Must run on the boot hart before any vector instruction. Safe to call on
/// targets without Zve: the `zve32x` block is omitted at compile time.
#[inline]
pub unsafe fn pre_main_init() {
    #[cfg(target_feature = "zve32x")]
    unsafe { init_zve_mstatus() };
}

#[cfg(target_feature = "zve32x")]
#[inline]
unsafe fn init_zve_mstatus() {
    use riscv::register::mstatus::{set_vs, VS};
    unsafe { set_vs(VS::Initial) };
}
