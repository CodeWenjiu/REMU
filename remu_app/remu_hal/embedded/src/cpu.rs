//! CPU / CSR setup before application logic.

/// Run once at the beginning of `main` (or call from [`crate::heap::init`]).
///
/// # Safety
///
/// Must run on the boot hart before any vector instruction.
#[inline]
pub unsafe fn pre_main_init() {
    #[cfg(target_feature = "zve32x")]
    {
        use riscv::register::mstatus::{VS, set_vs};
        unsafe { set_vs(VS::Initial) };
    }
}
