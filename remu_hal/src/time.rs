//! CLINT `mtime` — wall-clock style tick counter in remu.
//!
//! Matches [`remu_state::bus::device::clint`]: base `0x0200_0000`, `mtime` at offset `0xBFF8`,
//! **10 MHz** tick rate (derived from host `Instant` in the simulator).

/// CLINT MMIO base (default remu platform layout).
pub const CLINT_BASE: usize = 0x0200_0000;

/// Offset of 64-bit `mtime` (low 32 bits at `MTIME_LO_OFF`, high at `MTIME_LO_OFF + 4`).
pub const MTIME_LO_OFF: usize = 0xBFF8;

/// `mtime` advances at **10 MHz** in remu (see `mtime_ticks_from_elapsed_nanos` in `clint.rs`).
pub const MTIME_TICK_HZ: u64 = 10_000_000;

/// Read 64-bit CLINT `mtime` (RV32-safe: high/low/high until consistent).
#[inline]
pub fn read_mtime() -> u64 {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::ptr::read_volatile((CLINT_BASE + MTIME_LO_OFF) as *const u64)
    }
    #[cfg(target_arch = "riscv32")]
    loop {
        let hi1 =
            unsafe { core::ptr::read_volatile((CLINT_BASE + MTIME_LO_OFF + 4) as *const u32) };
        let lo = unsafe { core::ptr::read_volatile((CLINT_BASE + MTIME_LO_OFF) as *const u32) };
        let hi2 =
            unsafe { core::ptr::read_volatile((CLINT_BASE + MTIME_LO_OFF + 4) as *const u32) };
        if hi1 == hi2 {
            return ((hi1 as u64) << 32) | (lo as u64);
        }
    }
    #[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
    {
        0
    }
}
