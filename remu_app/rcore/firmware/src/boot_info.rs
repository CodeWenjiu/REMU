//! boot-info block layout, mirrored from `remu_state::bus::BootInfo` (the
//! simulator writes it; the firmware reads it via `a1`). The two sides must
//! stay in sync — it is a fixed ABI, not a shared crate: the firmware is
//! deliberately zero-dependency, and the layout is protocol (like the SBI EID
//! constants).

/// Magic: ASCII "REMU".
pub(crate) const MAGIC: u32 = 0x5245_4D55;

/// Version of this layout.
pub(crate) const VERSION: u32 = 1;

/// `mcause` values handled as SBI calls.
pub(crate) const MCAUSE_ENV_CALL_FROM_U: u64 = 8;
pub(crate) const MCAUSE_ENV_CALL_FROM_S: u64 = 9;

/// SBI legacy extension IDs (same numbers as the kernel's `sbi.rs` and the
/// SBI specification).
pub(crate) const EID_CONSOLE_PUTCHAR: u64 = 1;
pub(crate) const EID_SHUTDOWN: u64 = 8;

/// SBI_ERR_NOT_SUPPORTED (per the SBI spec).
pub(crate) const SBI_ERR_NOT_SUPPORTED: u64 = -2i64 as u64;

/// Boot info written by the simulator: resolved device map + handover target.
/// `repr(C)` — read as raw bytes off `a1`.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct BootInfo {
    pub(crate) magic: u32,
    pub(crate) version: u32,
    /// UART 16550 base (0 = not configured).
    pub(crate) uart_base: u64,
    /// SiFive test finisher base (0 = not configured).
    pub(crate) finisher_base: u64,
    /// CLINT base (0 = not configured).
    pub(crate) clint_base: u64,
    /// S-mode payload (kernel) entry point.
    pub(crate) kernel_entry: u64,
}

/// Fetch the boot info written by the simulator, `None` if the pointer or
/// magic/version is invalid.
pub(crate) fn boot_info() -> Option<BootInfo> {
    // Safety: `BOOT_INFO_ADDR` is written exactly once by `_start` before any
    // of this code runs; single-hart, no aliasing.
    let addr = unsafe { core::ptr::addr_of!(crate::BOOT_INFO_ADDR).read_unaligned() };
    if addr == 0 {
        return None;
    }
    // Safety: the simulator wrote a full `BootInfo` at this address before
    // reset; the magic check below validates the content.
    let info = unsafe { (addr as *const BootInfo).read_unaligned() };
    if info.magic == MAGIC && info.version == VERSION {
        Some(info)
    } else {
        None
    }
}
