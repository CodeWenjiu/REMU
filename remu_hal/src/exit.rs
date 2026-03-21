//! Program exit via SiFive test finisher.
//!
//! When running on remu, writing to this device signals program termination.

use crate::addresses::SIFIVE_TEST_FINISHER_BASE;

/// Magic value for success exit (remu reports ExitCode::Good).
pub const EXIT_SUCCESS: u32 = 0x5555;

/// Magic value for failure exit (remu reports ExitCode::Bad).
pub const EXIT_FAILURE: u32 = 0x3333;

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
