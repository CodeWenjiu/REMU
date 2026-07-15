//! Program exit via SiFive test finisher.

use crate::addresses::SIFIVE_TEST_FINISHER_BASE;

const EXIT_SUCCESS: u32 = 0x5555;
const EXIT_FAILURE: u32 = 0x3333;

/// Notify remu to exit successfully. Does not return.
#[inline(never)]
pub fn exit_success() -> ! {
    unsafe { core::ptr::write_volatile(SIFIVE_TEST_FINISHER_BASE as *mut u32, EXIT_SUCCESS) };
    loop {}
}

/// Notify remu to exit with failure. Does not return.
#[inline(never)]
pub fn exit_failure() -> ! {
    unsafe { core::ptr::write_volatile(SIFIVE_TEST_FINISHER_BASE as *mut u32, EXIT_FAILURE) };
    loop {}
}
