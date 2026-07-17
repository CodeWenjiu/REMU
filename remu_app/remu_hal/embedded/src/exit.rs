//! Program exit — platform selected at compile time via `--cfg platform_<name>`.

use crate::addresses::SIFIVE_TEST_FINISHER_BASE;

const EXIT_SUCCESS: u32 = 0x5555;
const EXIT_FAILURE: u32 = 0x3333;

/// Notify the runtime to exit successfully. Does not return.
#[inline(never)]
pub fn exit_success() -> ! {
    exit(EXIT_SUCCESS)
}

/// Notify the runtime to exit with failure. Does not return.
#[inline(never)]
pub fn exit_failure() -> ! {
    exit(EXIT_FAILURE)
}

#[cfg(platform_spike)]
unsafe extern "C" {
    static tohost: u64;
}

#[cfg(platform_spike)]
fn exit(code: u32) -> ! {
    // HTIF: write 1 for success, >= 3 for failure
    let htif = if code == EXIT_SUCCESS { 1u64 } else { 3u64 };
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    unsafe {
        core::ptr::write_volatile(core::ptr::addr_of!(tohost) as *mut u64, htif);
    }
    loop {}
}

#[cfg(not(platform_spike))]
fn exit(code: u32) -> ! {
    // SiFive test finisher (remu, QEMU)
    unsafe { core::ptr::write_volatile(SIFIVE_TEST_FINISHER_BASE as *mut u32, code) };
    loop {}
}
