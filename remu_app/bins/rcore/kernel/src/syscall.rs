//! Syscall dispatch (ch2): the kernel serves the user program's requests.
//!
//! Syscall numbers follow the Linux/RISC-V convention rCore uses:
//! `sys_write = 64` (a0=fd, a1=buf, a2=len), `sys_exit = 93` (a0=exit_code).
//! The kernel implements them on top of SBI: write -> console_putchar per
//! byte, exit -> shutdown.

use crate::sbi;
use crate::trap::TrapContext;

const SYS_WRITE: usize = 64;
const SYS_EXIT: usize = 93;

/// Caller-saved register indices in the trap context (ABI order).
const A0: usize = 10; // x10
const A1: usize = 11; // x11
const A2: usize = 12; // x12
const A7: usize = 17; // x17

/// Handle a syscall received via `ecall` from U mode.
/// On return, the trap epilogue `sret`s back to `sepc+4` (set here).
pub(crate) fn handle(ctx: &mut TrapContext) {
    // SBI calling convention expects the ecall to be skipped: the app's
    // `ecall` is the boundary instruction, so resume one instruction later.
    ctx.sepc += 4;
    match ctx.gpr[A7] {
        SYS_WRITE => {
            let (fd, buf, len) = (ctx.gpr[A0], ctx.gpr[A1], ctx.gpr[A2]);
            // ch2: only fd 1 (stdout) is supported.
            let written = if fd == 1 {
                write_stdout(buf as *const u8, len)
            } else {
                0
            };
            ctx.gpr[A0] = written;
        }
        SYS_EXIT => {
            let code = ctx.gpr[A0];
            // 0 = success, anything else = failure (finisher 0x5555 / 0x3333).
            sbi::shutdown(code != 0)
        }
        _other => {
            // Unknown syscall: per Linux convention, return -ENOSYS in a0.
            ctx.gpr[A0] = (-38isize) as usize;
        }
    }
}

/// Copy `len` bytes from user memory and print them via SBI console.
/// Returns the number of bytes written (on failure, fewer).
fn write_stdout(buf: *const u8, len: usize) -> usize {
    let mut written = 0;
    while written < len {
        // Safety: the user program's buffer is guest memory in the shared
        // address space; reads are always mapped (no MMU yet in remu/rcore
        // ch2). A fault would trap to the kernel, but we have no page
        // fault recovery yet.
        let byte = unsafe { buf.add(written).read_volatile() };
        sbi::console_putchar(byte);
        written += 1;
    }
    written
}
