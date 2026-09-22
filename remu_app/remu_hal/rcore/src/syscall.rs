//! Syscall ABI: `ecall` with the syscall ID in `a7`, up to three args in
//! `a0`-`a2`, return value in `a0` (Linux/RISC-V convention, mirrored by
//! `rcore_kernel::syscall`).
//!
//! The register-binding `asm!` is inherently RISC-V, so it is gated by
//! `target_arch`; hosted builds (workspace check only) get a stub that
//! returns `-1` — this crate is never *used* off-rcore.

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;

/// Perform a syscall: `id` in a7, `args` in a0..a2; returns a0.
#[cfg(target_arch = "riscv64")]
#[inline]
fn syscall(id: usize, args: [usize; 3]) -> isize {
    use core::arch::asm;
    let mut ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") id,
            options(nostack),
        );
    }
    ret
}

/// Host-check stub: no `ecall` on hosted targets.
#[cfg(not(target_arch = "riscv64"))]
#[inline]
fn syscall(_id: usize, _args: [usize; 3]) -> isize {
    -1
}

/// Write bytes to a file descriptor (ch2: only fd 1 = stdout works).
#[inline]
pub(crate) fn sys_write(fd: usize, buf: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buf.as_ptr() as usize, buf.len()])
}

/// Exit the program with a status code; does not return on success.
#[inline]
pub(crate) fn sys_exit(exit_code: i32) -> isize {
    syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0])
}
