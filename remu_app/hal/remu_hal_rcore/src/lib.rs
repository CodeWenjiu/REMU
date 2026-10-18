//! rcore U-mode arm of `remu_hal` — the S-mode-line counterpart of
//! `remu_hal_embedded` (MMIO) and `remu_hal_host` (stdout).
//!
//! Apps on the rcore platform (`--platform rcore`) run as U-mode user
//! programs under `rcore_kernel` and reach the machine through syscalls
//! (`ecall`, served by the kernel in S mode). This crate gives them the same
//! `remu_hal` API as everywhere else:
//!
//! - `_start` in `.text.entry` clears `.bss`, calls the app's `main` (the
//!   `#[remu_hal::entry]`-generated `extern "C"` symbol), then `sys_exit`s its
//!   return value;
//! - [`Uart16550`] writes to fd 1 via `sys_write` (drop-in for the MMIO
//!   device of the same name);
//! - [`exit_success`] / [`exit_failure`] map to `sys_exit`.
//!
//! All *platform* divergence is gone from this source: rcore is the only
//! target that matters (remu_hal pulls this crate in only on
//! `target_os = "rcore"`), and the remaining `#[cfg(target_arch = "riscv64")]`
//! guards are just the RISC-V assembly chunks themselves — host builds keep
//! a stubbed syscall so `cargo check --workspace` stays green.

#![no_std]

remu_macro::mod_prv!(exit, stdout, syscall);

pub use exit::{exit_failure, exit_success};
pub use stdout::Uart16550;

#[cfg(target_arch = "riscv64")]
use core::arch::global_asm;

/// Linker symbol helpers for the user program's `.bss` range.
#[cfg(target_arch = "riscv64")]
macro_rules! linker_symbol_addr {
    ($symbol:path) => {
        unsafe { ($symbol as *const ()).addr() }
    };
}

// U-mode entry: clear `.bss`, run `main`, exit with its return value. The
// batch system does not zero `.bss` for us yet.
#[cfg(target_arch = "riscv64")]
global_asm!(
    ".section .text.entry",
    ".globl _start",
    "_start:",
    "  call clear_bss",
    "  call main",
    "  li a7, {exit}",
    "  ecall",
    "1:",
    "  j 1b", // unreachable after sys_exit
    exit = const 93,
);

/// Zero the `.bss` section.
#[cfg(target_arch = "riscv64")]
#[unsafe(no_mangle)]
extern "C" fn clear_bss() {
    unsafe extern "C" {
        static sbss: u8;
        static ebss: u8;
    }
    let start = linker_symbol_addr!(sbss);
    let end = linker_symbol_addr!(ebss);
    for addr in start..end {
        // Safety: sbss..ebss is the real .bss range in the final image.
        unsafe { (addr as *mut u8).write_volatile(0) };
    }
}

// `main` is provided by the app's own crate. The `#[remu_hal::entry]` macro
// generates it as `#[unsafe(no_mangle)] extern "C"`.
#[allow(dead_code)]
unsafe extern "C" {
    fn main() -> i32;
}

/// Panic handler for U-mode programs: report the message and exit(1) instead
/// of hanging silently. Formats into a stack buffer (no heap in user programs).
///
/// `#[cfg(target_arch = "riscv64")]`: a panic handler must exist exactly once
/// in the final binary, so host builds (which use std's own handler) must not
/// see it.
#[cfg(target_arch = "riscv64")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;
    struct Buf<'a> {
        data: &'a mut [u8],
        len: usize,
    }
    impl Write for Buf<'_> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let room = &mut self.data[self.len..];
            for (i, b) in s.bytes().take(room.len()).enumerate() {
                room[i] = b;
            }
            self.len += s.len().min(room.len());
            Ok(())
        }
    }
    let mut buf = [0u8; 128];
    let len = {
        let mut w = Buf {
            data: &mut buf,
            len: 0,
        };
        let _ = write!(w, "panicked at {}", info.message());
        if let Some(l) = info.location() {
            let _ = write!(w, " ({}:{})", l.file(), l.line());
        }
        w.len
    };
    let _ = syscall::sys_write(1, &buf[..len]);
    let _ = syscall::sys_exit(1);
    loop {
        core::hint::spin_loop()
    }
}
