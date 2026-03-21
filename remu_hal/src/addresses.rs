//! Device MMIO base addresses (must match remu default configuration).

/// UART 16550 base address (default: uart16550@0x1000_0000).
pub const UART16550_BASE: usize = 0x1000_0000;

/// SiFive test finisher base address (default: sifive_test_finisher@0x0010_0000).
/// Writing 0x5555 = success exit, 0x3333 = fail exit.
pub const SIFIVE_TEST_FINISHER_BASE: usize = 0x0010_0000;
