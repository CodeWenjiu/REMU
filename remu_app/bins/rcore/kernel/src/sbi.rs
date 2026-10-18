//! SBI (Supervisor Binary Interface) calls — the S-mode kernel's door to the
//! M-mode firmware.
//!
//! Only the two **legacy** extensions chapter 1 needs are wrapped:
//!
//! | EID | Function            |
//! |-----|---------------------|
//! | `1` | `console_putchar`   |
//! | `8` | `shutdown`          |
//!
//! The IDs are part of the SBI specification (shared protocol constants), so
//! the firmware side (remu's `remu_firmware` / QEMU's OpenSBI) hardcodes the
//! same numbers — the same "two independent implementations of one spec"
//! convention the project already uses for MMIO register maps.

/// SBI extension ID: legacy console output.
const EID_CONSOLE_PUTCHAR: usize = 1;
/// SBI extension ID: legacy system shutdown.
const EID_SHUTDOWN: usize = 8;

/// Perform an SBI call and return the firmware's error code (`a0`).
///
/// `a7` carries the extension ID; `a0` is both the first argument and the
/// return value, matching the SBI calling convention.
#[inline]
fn sbi_call(eid: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let ret: usize;
    let _err: usize;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") arg0 => ret,
            inlateout("a1") arg1 => _err,
            in("a2") arg2,
            in("a7") eid,
            options(nostack),
        );
    }
    ret
}

/// Print one byte to the firmware console.
#[inline]
pub(crate) fn console_putchar(c: u8) {
    let _ = sbi_call(EID_CONSOLE_PUTCHAR, c as usize, 0, 0);
}

/// Print a string to the firmware console.
pub(crate) fn console_puts(s: &str) {
    for byte in s.bytes() {
        console_putchar(byte);
    }
}

/// Ask the firmware to shut down (or, when `fail` is set, report a failure).
pub(crate) fn shutdown(fail: bool) -> ! {
    // Legacy shutdown: a0=0 success, a0=1 failure; does not return on success.
    let _ = sbi_call(EID_SHUTDOWN, usize::from(fail), 0, 0);
    // If the firmware declined to shut down, fall back to a halt loop so the
    // kernel never "returns" into nothing.
    loop {
        core::hint::spin_loop();
    }
}
