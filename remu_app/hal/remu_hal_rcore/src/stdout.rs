//! U-mode stdout writer, API-compatible with the embedded `Uart16550`.

use crate::syscall;

/// U-mode stdout: `sys_write` of fd 1, one syscall per `write_str` call.
pub struct Uart16550;

impl Uart16550 {
    #[inline]
    pub const fn default_base() -> Self {
        Uart16550
    }
}

impl core::fmt::Write for Uart16550 {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        syscall::sys_write(1, s.as_bytes());
        Ok(())
    }
}
