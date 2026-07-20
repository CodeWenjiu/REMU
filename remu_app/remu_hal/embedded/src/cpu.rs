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

    // Copy compile-time application arguments to the known address.
    let data: &[u8] = include!(concat!(env!("OUT_DIR"), "/app_args_data.rs"));
    let len = data.len();
    if len > 1 {
        let dst = crate::app_args::APP_ARGS_BASE as *mut u8;
        unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), dst, len) };
    }
}
