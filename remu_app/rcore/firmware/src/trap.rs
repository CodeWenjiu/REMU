//! Machine trap vector: save/restore the full GPR context and dispatch SBI
//! calls from S mode (the kernel's `ecall`).
//!
//! Layout of the trap frame (288 bytes):
//!
//! ```text
//!  0     x1    (ra)        ...  saved GPRs x1..x31
//!  248   mepc              ...  (may be modified by the handler)
//!  256   mcause
//!  264   mtval
//!  272   (padding / scratch)
//! ```

use crate::boot_info::{self, BootInfo};

#[cfg(target_os = "none")]
use core::arch::global_asm;

/// Trap frame pushed by `_trap_entry`; `mepc` may be modified by the handler
/// (e.g. `+4` past a served `ecall`), and the entry restores it.
#[repr(C)]
pub(crate) struct TrapFrame {
    // GPRs x1..x31 (x0 is always zero). Index N is register x(N+1).
    pub(crate) gpr: [u64; 31],
    pub(crate) mepc: u64,
    pub(crate) mcause: u64,
    pub(crate) mtval: u64,
}

#[cfg(target_os = "none")]
global_asm!(
    ".section .text",
    ".globl _trap_entry",
    ".balign 4",
    "_trap_entry:",
    // Save 31 GPRs (x0 is read-only zero) + mepc/mcause/mtval.
    "  addi sp, sp, -288",
    "  sd x1, 0(sp)",
    "  sd x2, 8(sp)",
    "  sd x3, 16(sp)",
    "  sd x4, 24(sp)",
    "  sd x5, 32(sp)",
    "  sd x6, 40(sp)",
    "  sd x7, 48(sp)",
    "  sd x8, 56(sp)",
    "  sd x9, 64(sp)",
    "  sd x10, 72(sp)",
    "  sd x11, 80(sp)",
    "  sd x12, 88(sp)",
    "  sd x13, 96(sp)",
    "  sd x14, 104(sp)",
    "  sd x15, 112(sp)",
    "  sd x16, 120(sp)",
    "  sd x17, 128(sp)",
    "  sd x18, 136(sp)",
    "  sd x19, 144(sp)",
    "  sd x20, 152(sp)",
    "  sd x21, 160(sp)",
    "  sd x22, 168(sp)",
    "  sd x23, 176(sp)",
    "  sd x24, 184(sp)",
    "  sd x25, 192(sp)",
    "  sd x26, 200(sp)",
    "  sd x27, 208(sp)",
    "  sd x28, 216(sp)",
    "  sd x29, 224(sp)",
    "  sd x30, 232(sp)",
    "  sd x31, 240(sp)",
    "  csrr t0, mepc",
    "  sd t0, 248(sp)",
    "  csrr t0, mcause",
    "  sd t0, 256(sp)",
    "  csrr t0, mtval",
    "  sd t0, 264(sp)",
    // Dispatch to Rust with the frame pointer in a0.
    "  mv a0, sp",
    "  call trap_handler",
    // Restore mepc (the handler may have advanced it past the ecall).
    "  ld t0, 248(sp)",
    "  csrw mepc, t0",
    "  ld x1, 0(sp)",
    "  ld x2, 8(sp)",
    "  ld x3, 16(sp)",
    "  ld x4, 24(sp)",
    "  ld x5, 32(sp)",
    "  ld x6, 40(sp)",
    "  ld x7, 48(sp)",
    "  ld x8, 56(sp)",
    "  ld x9, 64(sp)",
    "  ld x10, 72(sp)",
    "  ld x11, 80(sp)",
    "  ld x12, 88(sp)",
    "  ld x13, 96(sp)",
    "  ld x14, 104(sp)",
    "  ld x15, 112(sp)",
    "  ld x16, 120(sp)",
    "  ld x17, 128(sp)",
    "  ld x18, 136(sp)",
    "  ld x19, 144(sp)",
    "  ld x20, 152(sp)",
    "  ld x21, 160(sp)",
    "  ld x22, 168(sp)",
    "  ld x23, 176(sp)",
    "  ld x24, 184(sp)",
    "  ld x25, 192(sp)",
    "  ld x26, 200(sp)",
    "  ld x27, 208(sp)",
    "  ld x28, 216(sp)",
    "  ld x29, 224(sp)",
    "  ld x30, 232(sp)",
    "  ld x31, 240(sp)",
    "  addi sp, sp, 288",
    "  mret",
);

/// SBI call service: `a0` = first arg / return, `a7` = extension ID.
/// The trap frame GPRs are indexed as x1..x31, so x10 = frame.gpr[9] (a0),
/// x17 = frame.gpr[16] (a7).
#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
extern "C" fn trap_handler(frame: &mut TrapFrame) {
    let cause = frame.mcause & 0x7FFF_FFFF;
    if cause == boot_info::MCAUSE_ENV_CALL_FROM_S || cause == boot_info::MCAUSE_ENV_CALL_FROM_U {
        // SBI call: serve from the boot-info device map.
        let ret = serve_sbi(frame);
        frame.gpr[9] = ret; // a0
        frame.mepc = frame.mepc.wrapping_add(4); // skip the ecall
    } else {
        // Unknown machine trap: nothing sensible to do without the kernel's
        // handlers; hang (the panic handler may have printed first if it is
        // a Rust panic).
        loop {
            core::hint::spin_loop();
        }
    }
}

/// Serve one SBI call per the legacy spec; returns the value for `a0`.
#[cfg(target_os = "none")]
fn serve_sbi(frame: &TrapFrame) -> u64 {
    let eid = frame.gpr[16]; // x17 = a7
    let info = boot_info::boot_info();
    match (eid, info) {
        (boot_info::EID_CONSOLE_PUTCHAR, Some(info)) => {
            let ch = frame.gpr[9] as u8; // x10 = a0: the character
            uart_putchar(info, ch);
            0
        }
        (boot_info::EID_SHUTDOWN, Some(info)) => {
            let fail = frame.gpr[9] != 0; // x10 = a0: 0 = success, else fail
            finisher_exit(info, fail)
        }
        _ => boot_info::SBI_ERR_NOT_SUPPORTED,
    }
}

/// Write one character to the UART THR (0 = base). No-op if the boot info
/// carries no UART.
#[cfg(target_os = "none")]
fn uart_putchar(info: BootInfo, ch: u8) {
    if info.uart_base != 0 {
        // Safety: the UART base comes from the simulator's device map and is
        // a valid MMIO address for this hart.
        unsafe { core::ptr::write_volatile(info.uart_base as *mut u8, ch) };
    }
}

/// Write the finisher register; never returns on a real exit.
#[cfg(target_os = "none")]
fn finisher_exit(info: BootInfo, fail: bool) -> u64 {
    let code: u32 = if fail { 0x3333 } else { 0x5555 };
    if info.finisher_base != 0 {
        // Safety: the finisher base comes from the simulator's device map.
        // The write makes the simulator end the run with the exit code.
        unsafe { core::ptr::write_volatile(info.finisher_base as *mut u32, code) };
    }
    // If the write did not end the run (no finisher configured), hang.
    loop {
        core::hint::spin_loop();
    }
    #[allow(unreachable_code)]
    boot_info::SBI_ERR_NOT_SUPPORTED
}

/// Best-effort panic print to the UART (used by the kernel panic handler).
#[cfg(target_os = "none")]
pub(crate) fn panic_print(msg: &str) {
    if let Some(info) = boot_info::boot_info() {
        for b in msg.bytes() {
            uart_putchar(info, b);
        }
    }
}

/// Host-check stub: `trap_handler` compiles out on hosted targets.
#[cfg(not(target_os = "none"))]
pub(crate) fn panic_print(_msg: &str) {}
