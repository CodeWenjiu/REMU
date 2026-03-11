//! Unified statistics interface: common stats (e.g. inst count) + platform-specific (e.g. cycle count, IPC).

use clap::Subcommand;

/// Context passed to platform_stats so platforms can compute derived stats (e.g. IPC).
#[derive(Debug, Clone)]
pub struct StatContext {
    /// Instruction count from Harness.
    pub inst_count: u64,
}

#[derive(Debug, Clone)]
pub enum StatEntry {
    /// Instruction count (all platforms; maintained by Harness).
    InstCount(u64),
    /// Clock cycle count (nzea etc.; not available on remu).
    CycleCount(u64),
    /// Instructions per cycle (derived; nzea etc.).
    Ipc(f64),
}

impl StatEntry {
    pub fn name(&self) -> &'static str {
        match self {
            Self::InstCount(_) => "inst_count",
            Self::CycleCount(_) => "cycle_count",
            Self::Ipc(_) => "ipc",
        }
    }

    pub fn format(&self) -> String {
        match self {
            Self::InstCount(v) => format!("{}", v),
            Self::CycleCount(v) => format!("{}", v),
            Self::Ipc(v) => format!("{:.4}", v),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum StatCmd {
    /// Print all statistics
    Print,
}
