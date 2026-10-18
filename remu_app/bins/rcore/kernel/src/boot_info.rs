//! Boot-info block layout, mirrored from `remu_state::bus::BootInfo` (the
//! simulator writes it; the firmware reads it at reset via `a1`, the kernel
//! reads it from the same fixed address after handover). The three sides must
//! stay in sync — it is a fixed ABI, not a shared crate: kernel and firmware
//! are deliberately zero-dependency, and the layout is protocol (like the SBI
//! EID constants).

/// Fixed address of the boot-info block (must match
/// `remu_state::bus::BOOT_INFO_BASE`).
pub(crate) const BOOT_INFO_BASE: usize = 0x87FF_E000;

/// Magic: ASCII "REMU".
const MAGIC: u32 = 0x5245_4D55;

/// Version of this layout.
const VERSION: u32 = 1;

/// Boot info written by the simulator: resolved device map + handover targets.
/// `repr(C)` — read as raw bytes.
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
    /// U-mode payload (user program) entry point (0 = none loaded yet).
    pub(crate) app_entry: u64,
}

/// Fetch the boot info from its fixed address, `None` if the magic/version
/// is invalid.
pub(crate) fn boot_info() -> Option<BootInfo> {
    // Safety: the simulator wrote a full `BootInfo` at this address before
    // reset; the magic check below validates the content.
    let info = unsafe { (BOOT_INFO_BASE as *const BootInfo).read_unaligned() };
    if info.magic == MAGIC && info.version == VERSION {
        Some(info)
    } else {
        None
    }
}
