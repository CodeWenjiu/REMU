//! Device MMIO base addresses (must match remu default configuration).

/// UART 16550 base address (default: uart16550@0x1000_0000).
pub(crate) const UART16550_BASE: usize = 0x1000_0000;

/// Display device base address (default: display@0x8800_0000).
pub(crate) const DISPLAY_BASE: usize = 0x8800_0000;

/// SiFive test finisher base address (default: sifive_test_finisher@0x0010_0000).
#[cfg(not(platform_spike))]
pub(crate) const SIFIVE_TEST_FINISHER_BASE: usize = 0x0010_0000;
