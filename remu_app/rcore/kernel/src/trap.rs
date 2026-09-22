//! S-mode trap handling (ch2): the kernel runs the user program in U mode and
//! receives its `ecall` here. `stvec` points at `_trap_entry`; the entry saves
//! the U-mode context onto the kernel stack (via `sscratch`), dispatches to
//! [`trap_handler`], then restores and `sret`s back.
//!
//! Trap frame layout (32 GPRs + sepc/scause/stval + padding, 8-byte each):
//! x0..x31 at 0..256, sepc 256, scause 264, stval 272, size 288 bytes.

use crate::boot_info;
use crate::syscall;

use core::arch::global_asm;

/// Kernel stack for trap handling. The user program runs on its own stack;
/// `sscratch` holds the top of *this* stack while the kernel is in U mode.
const TRAP_STACK_SIZE: usize = 16 * 1024;

global_asm!(
    ".section .bss.stack",
    ".globl _trap_stack_bottom",
    "_trap_stack_bottom:",
    ".space {size}",
    ".globl _trap_stack_top",
    "_trap_stack_top:",
    size = const TRAP_STACK_SIZE,
);

global_asm!(
    ".section .text",
    ".globl _trap_entry",
    ".balign 4",
    "_trap_entry:",
    // swap sp with sscratch (kernel stack top <-> user sp). After this,
    // sp = kernel stack, sscratch = user sp.
    "  csrrw sp, sscratch, sp",
    // Save 32 GPRs. x2's slot gets the *user* sp, read back from sscratch,
    // because x2 (=sp) now points at the kernel stack.
    "  addi sp, sp, -288",
    "  sd x0, 0(sp)",
    "  sd x1, 8(sp)",
    "  csrr t0, sscratch",
    "  sd t0, 16(sp)",
    "  sd x3, 24(sp)",
    "  sd x4, 32(sp)",
    "  sd x5, 40(sp)",
    "  sd x6, 48(sp)",
    "  sd x7, 56(sp)",
    "  sd x8, 64(sp)",
    "  sd x9, 72(sp)",
    "  sd x10, 80(sp)",
    "  sd x11, 88(sp)",
    "  sd x12, 96(sp)",
    "  sd x13, 104(sp)",
    "  sd x14, 112(sp)",
    "  sd x15, 120(sp)",
    "  sd x16, 128(sp)",
    "  sd x17, 136(sp)",
    "  sd x18, 144(sp)",
    "  sd x19, 152(sp)",
    "  sd x20, 160(sp)",
    "  sd x21, 168(sp)",
    "  sd x22, 176(sp)",
    "  sd x23, 184(sp)",
    "  sd x24, 192(sp)",
    "  sd x25, 200(sp)",
    "  sd x26, 208(sp)",
    "  sd x27, 216(sp)",
    "  sd x28, 224(sp)",
    "  sd x29, 232(sp)",
    "  sd x30, 240(sp)",
    "  sd x31, 248(sp)",
    "  csrr t0, sepc",
    "  sd t0, 256(sp)",
    "  csrr t0, scause",
    "  sd t0, 264(sp)",
    "  csrr t0, stval",
    "  sd t0, 272(sp)",
    // Kernel is now in S mode with the kernel stack; point sscratch back at
    // the kernel stack top so the next trap swaps correctly.
    "  la t0, _trap_stack_top",
    "  csrw sscratch, t0",
    "  mv a0, sp",
    "  call trap_handler",
    // Restore sepc (handler may have advanced it past the ecall).
    "  ld t0, 256(sp)",
    "  csrw sepc, t0",
    // Restore GPRs. x2 (sp) is *not* loaded here: restoring it would clobber
    // the kernel sp we still need for the remaining loads. Instead the user
    // sp is parked in sscratch and swapped in by the final csrrw below.
    "  ld t0, 16(sp)",
    "  csrw sscratch, t0",
    "  ld x1, 8(sp)",
    "  ld x3, 24(sp)",
    "  ld x4, 32(sp)",
    "  ld x5, 40(sp)",
    "  ld x6, 48(sp)",
    "  ld x7, 56(sp)",
    "  ld x8, 64(sp)",
    "  ld x9, 72(sp)",
    "  ld x10, 80(sp)",
    "  ld x11, 88(sp)",
    "  ld x12, 96(sp)",
    "  ld x13, 104(sp)",
    "  ld x14, 112(sp)",
    "  ld x15, 120(sp)",
    "  ld x16, 128(sp)",
    "  ld x17, 136(sp)",
    "  ld x18, 144(sp)",
    "  ld x19, 152(sp)",
    "  ld x20, 160(sp)",
    "  ld x21, 168(sp)",
    "  ld x22, 176(sp)",
    "  ld x23, 184(sp)",
    "  ld x24, 192(sp)",
    "  ld x25, 200(sp)",
    "  ld x26, 208(sp)",
    "  ld x27, 216(sp)",
    "  ld x28, 224(sp)",
    "  ld x29, 232(sp)",
    "  ld x30, 240(sp)",
    "  ld x31, 248(sp)",
    "  addi sp, sp, 288",
    // sp is now the kernel stack top again; swap in the user sp (sscratch).
    "  csrrw sp, sscratch, sp",
    "  sret",
);

/// Trap context handed to [`trap_handler`]: all GPRs + the trap CSRs.
/// GPRs are indexed by ABI register number (x0..x31).
#[repr(C)]
#[derive(Debug)]
pub(crate) struct TrapContext {
    pub(crate) gpr: [usize; 32],
    pub(crate) sepc: usize,
    pub(crate) scause: usize,
    pub(crate) stval: usize,
}

/// Trap cause: environment call from U mode (ecall).
const CAUSE_ENV_CALL_FROM_U: usize = 8;

/// Dispatch a trap taken in S mode (from the user program in U mode).
///
/// ch2 supports exactly one trap source: the user program's `ecall` (syscall).
/// Anything else is a kernel panic (the kernel has no other handler yet).
#[unsafe(no_mangle)]
extern "C" fn trap_handler(ctx: &mut TrapContext) {
    let cause = ctx.scause & !(1usize << (usize::BITS - 1)); // strip interrupt bit
    if cause == CAUSE_ENV_CALL_FROM_U {
        syscall::handle(ctx);
    } else {
        panic!(
            "unhandled trap: scause={:#x} sepc={:#x} stval={:#x}",
            ctx.scause, ctx.sepc, ctx.stval
        );
    }
}

/// Enter the user program (`app_entry` from the boot info) via `sret`.
/// `app_entry == 0` means no user program was loaded: print and halt.
///
/// Before `sret` we install a user stack (fixed top just below the app
/// region) and point `sscratch` at the kernel trap stack top, so the first
/// U-mode `ecall` swaps to the kernel stack correctly.
pub(crate) fn enter_user() -> ! {
    let info = boot_info::boot_info().expect("boot info missing");
    if info.app_entry == 0 {
        crate::console_puts("no user program loaded (pass --app)\n");
        crate::shutdown(false)
    }
    // Prepare the U-mode context: sepc = entry, SPP = 0 (U), user sp = stack
    // top (app region base - 8 KiB, below the program), sscratch = kernel trap
    // stack top.
    unsafe {
        core::arch::asm!(
            "csrw sepc, {entry}",
            "csrc sstatus, {spp}", // clear SPP -> return to U mode
            "li sp, {user_sp}",
            "csrw sscratch, {stack}",
            "sret",
            entry = in(reg) info.app_entry as usize,
            spp = in(reg) 1usize << 8,
            user_sp = const APP_USER_STACK_TOP,
            stack = in(reg) _trap_stack_top_addr(),
            options(nomem, nostack),
        );
    }
    unreachable!()
}

/// User stack top: just below the user program region (0x8040_0000), so the
/// app's stack grows down from 0x8040_0000 - 8 KiB.
pub(crate) const APP_USER_STACK_TOP: usize = 0x8040_0000 - 8 * 1024;

/// Address of the kernel trap stack top (for `sscratch`).
fn _trap_stack_top_addr() -> usize {
    unsafe extern "C" {
        static _trap_stack_top: u8;
    }
    core::ptr::addr_of!(_trap_stack_top) as usize
}
