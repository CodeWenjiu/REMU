//! Application arguments bridge between remu simulator and bare-metal programs.
//!
//! Reads a null-terminated string written by the simulator to
//! `APP_ARGS_BASE` (top of RAM - 4 KiB) before boot.
//!
//! Returns an empty string if no arguments were passed.

/// Address where the simulator writes application arguments before boot.
const APP_ARGS_BASE: usize = 0x87FF_F000;
const APP_ARGS_MAX: usize = 4096;

/// Return the application argument string, or an empty string if none.
pub fn app_args() -> &'static str {
    let ptr = APP_ARGS_BASE as *const u8;
    let first = unsafe { core::ptr::read_volatile(ptr) };
    if first == 0 {
        return "";
    }
    let mut len = 0usize;
    unsafe {
        while len < APP_ARGS_MAX && ptr.add(len).read_volatile() != 0 {
            len += 1;
        }
        core::str::from_utf8_unchecked(core::slice::from_raw_parts(ptr, len))
    }
}
