//! remu firmware — the M-mode program that serves the kernel's SBI calls.
//!
//! This is the "OpenSBI role" for remu: a real firmware image loaded with
//! `--firmware` that boots at the reset PC (0x8000_0000, the QEMU `virt`
//! reset region), installs its own `mtvec`, hands over to the S-mode kernel
//! with `mret`, and then serves the kernel's `ecall` (SBI) from the trap
//! vector — exactly what OpenSBI does on QEMU.
//!
//! Per the RISC-V boot convention the reset state carries `a0` = hartid and
//! `a1` = boot-info pointer. The boot info is written by the simulator (see
//! `remu_state::bus::write_boot_info`) and contains the *resolved* device map
//! (UART / finisher / CLINT bases) plus the kernel entry point — so this
//! firmware hardcodes no device addresses, matching the project rule that the
//! device map is runtime configuration.
//!
//! This crate is a bare-metal binary for `riscv64im-unknown-none-elf`, gated
//! behind the `bare-metal` feature (see Cargo.toml): hosted builds
//! (`cargo check --workspace`) skip the binary entirely, so there is no
//! conditional compilation here — `target_os = "none"` is the only build.

#![no_std]
#![no_main]

mod boot_info;
mod trap;

use core::arch::global_asm;
use core::panic::PanicInfo;

/// Firmware stack, 16 KiB (trap frames are 288 bytes; the kernel runs on its
/// own stack once handed over).
const BOOT_STACK_SIZE: usize = 16 * 1024;

/// Boot-info pointer handed over in `a1` at reset; stashed by `_start`. Read
/// (never written again) by the trap handler.
#[unsafe(no_mangle)]
static mut BOOT_INFO_ADDR: usize = 0;

global_asm!(
    ".section .bss.stack",
    ".globl _boot_stack_bottom",
    "_boot_stack_bottom:",
    ".space {size}",
    ".globl _boot_stack_top",
    "_boot_stack_top:",
    size = const BOOT_STACK_SIZE,
);

// _start: stack up, stash a1 (boot info), install mtvec, then hand over to
// the S-mode kernel (MEPC = kernel entry, MPP = S, mret).
global_asm!(
    ".section .text.entry",
    ".globl _start",
    "_start:",
    "  la sp, _boot_stack_top",
    // Stash the boot-info pointer (a1) for the trap handler.
    "  la t0, BOOT_INFO_ADDR",
    "  sd a1, 0(t0)",
    // Install the trap vector before anything can fault.
    "  la t0, _trap_entry",
    "  csrw mtvec, t0",
    // Validate the boot info; halt with a silent wfi loop on mismatch.
    // magic is u32 at offset 0 (next u32 is version) — read it 32-bit.
    "  lw t0, 0(a1)",
    "  li t1, {magic}",
    "  bne t0, t1, 2f",
    // Hand over: MEPC = kernel entry, MPP = S, then mret.
    // Write mstatus outright (not `csrs`): the reset value has MPP=M already
    // (bit 12), so blindly setting bit 11 would leave MPP=M and the kernel
    // would trap as M-mode. 1<<11 = MPP=01 (S), MIE=MPIE=0.
    "  ld t0, 32(a1)",
    "  csrw mepc, t0",
    "  li t0, 1",
    "  slli t0, t0, 11", // mstatus.MPP = 01 (S)
    "  csrw mstatus, t0",
    "  mret",
    "2:",
    "  wfi",
    "  j 2b",
    magic = const boot_info::MAGIC as u64,
);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(msg) = info.message().as_str() {
        // Best effort: print to the UART if the boot info carried one.
        trap::panic_print(msg);
    }
    loop {
        core::hint::spin_loop();
    }
}
