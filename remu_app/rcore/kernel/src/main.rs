//! rcore kernel — chapter 1: first instruction and SBI services.
//!
//! Layout: `_start` (assembly) is placed in `.text.entry` so it sits at the
//! very front of the image; the linker script pins the image at `0x8020_0000`.
//! `_start` sets up the boot stack and calls [`rust_main`].
//!
//! Output and shutdown go through SBI (Supervisor Binary Interface): `ecall`
//! with the legacy console/shutdown extension IDs. The firmware serves them:
//! remu's `remu_firmware` (`--firmware`) on remu, OpenSBI on QEMU — the
//! kernel code is identical on both.
//!
//! This crate is a bare-metal binary for `riscv64im-unknown-none-elf`. It is
//! still a workspace member so `-p` builds and shared lints work uniformly; on
//! hosted targets (`cargo check --workspace` runs on the host) everything but a
//! diagnostic stub compiles out, keeping the workspace check green.

#![cfg_attr(target_os = "none", no_std, no_main)]

#[cfg(target_os = "none")]
mod sbi;

#[cfg(target_os = "none")]
use core::arch::global_asm;
#[cfg(target_os = "none")]
use core::panic::PanicInfo;

/// Boot stack, 16 KiB (chapter 1 only needs enough room for a handful of
/// frames; later chapters grow this).
#[cfg(target_os = "none")]
const BOOT_STACK_SIZE: usize = 16 * 1024;

#[cfg(target_os = "none")]
global_asm!(
    ".section .bss.stack",
    ".globl _boot_stack_bottom",
    "_boot_stack_bottom:",
    ".space {size}",
    ".globl _boot_stack_top",
    "_boot_stack_top:",
    size = const BOOT_STACK_SIZE,
);

#[cfg(target_os = "none")]
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

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
extern "C" fn rust_main() -> ! {
    sbi::console_puts("rcore kernel: chapter 1\n");
    sbi::console_puts("hello from S mode via SBI\n");

    sbi::shutdown(false)
}

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    sbi::console_puts("kernel panic: ");
    if let Some(msg) = info.message().as_str() {
        sbi::console_puts(msg);
    }
    sbi::console_puts("\n");
    sbi::shutdown(true)
}

/// Host-check stub: the kernel cannot run on a hosted target. This exists so
/// `cargo check --workspace` (which runs on the host) stays green and a
/// mis-targeted `cargo run` fails with a pointer to the right command.
#[cfg(not(target_os = "none"))]
fn main() {
    eprintln!(
        "rcore_kernel is a bare-metal binary (riscv64im-unknown-none-elf); \
         build/run it with `just build-os` / `just run-os`"
    );
    std::process::exit(1);
}
