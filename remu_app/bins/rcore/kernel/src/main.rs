//! rcore kernel — chapter 2: batch system (run a user program in U mode).
//!
//! Layout: `_start` (assembly) is placed in `.text.entry` so it sits at the
//! very front of the image; the linker script pins the image at `0x8020_0000`.
//! `_start` sets up the boot stack and calls [`rust_main`].
//!
//! Flow: the firmware (M mode) hands over with `mret`; the kernel reports
//! readiness over SBI, installs its own `stvec`, reads the user-program entry
//! from the boot info (`app_entry`), and `sret`s into U mode. The user
//! program's `ecall` traps back to `stvec` (S mode, `trap.rs`), which serves
//! syscalls (`syscall.rs`) on top of SBI — the same layered service OpenSBI
//! provides on QEMU, with the simulator's `remu_firmware` in the M-mode role.
//!
//! This crate is a bare-metal binary for `riscv64im-unknown-none-elf`, gated
//! behind the `bare-metal` feature (see Cargo.toml): hosted builds
//! (`cargo check --workspace`) skip the binary entirely, so there is no
//! conditional compilation here — `target_os = "none"` is the only build.

#![no_std]
#![no_main]

mod boot_info;
mod sbi;
mod syscall;
mod trap;

pub(crate) use sbi::{console_puts, shutdown};

use core::arch::global_asm;
use core::panic::PanicInfo;

/// Boot stack, 16 KiB (the trap handler uses its own stack; this one only
/// carries `rust_main` up to the first `sret`).
const BOOT_STACK_SIZE: usize = 16 * 1024;

global_asm!(
    ".section .bss.stack",
    ".globl _boot_stack_bottom",
    "_boot_stack_bottom:",
    ".space {size}",
    ".globl _boot_stack_top",
    "_boot_stack_top:",
    size = const BOOT_STACK_SIZE,
);

global_asm!(
    ".section .text.entry",
    ".globl _start",
    "_start:",
    // sp = stack top; stack grows down.
    "  la sp, _boot_stack_top",
    "  call rust_main",
    // rust_main is `-> !`, but stay safe if it ever returns.
    "1:",
    "  j 1b",
);

#[unsafe(no_mangle)]
extern "C" fn rust_main() -> ! {
    sbi::console_puts("rcore kernel: chapter 2\n");

    // Install the S-mode trap vector, then hand control to the user program.
    // Safety: `_trap_entry` is in this binary and never moves.
    unsafe {
        core::arch::asm!("la t0, _trap_entry", "csrw stvec, t0",);
    }
    trap::enter_user()
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    sbi::console_puts("kernel panic: ");
    if let Some(msg) = info.message().as_str() {
        sbi::console_puts(msg);
    }
    sbi::console_puts("\n");
    sbi::shutdown(true)
}
