//! Unified statistics interface: common stats (e.g. inst count) + platform-specific (e.g. cycle count, IPC).

use clap::Subcommand;
use remu_types::StatKind;

/// Which statistics the caller wants to see. Filtering is applied by the
/// platform implementation (it owns the raw signals and derived rules).
#[derive(Debug, Clone)]
pub enum StatFilter {
    /// Everything: all raw counters plus all derived entries.
    All,
    /// Raw counters only.
    Raw,
    /// A derived group: the group's dependency counters plus its derived
    /// entries (group names come from the platform's derive-rule table,
    /// e.g. "ipc", "bp").
    Group(String),
}

#[derive(Debug, Clone)]
pub enum StatEntry {
    /// Raw statistic: a platform counter (e.g. nzea RTL VPI signal), displayed
    /// with its platform-given name.
    Named { name: String, value: String },
    /// Derived statistic: computed from raw counters (e.g. IPC, mispredict rate).
    Derived { name: String, value: String },
}

impl StatEntry {
    pub fn name(&self) -> String {
        match self {
            Self::Named { name, .. } | Self::Derived { name, .. } => name.clone(),
        }
    }

    pub fn format(&self) -> String {
        match self {
            Self::Named { value, .. } | Self::Derived { value, .. } => value.clone(),
        }
    }

    /// Whether this entry is a raw counter or a derived value.
    pub fn kind(&self) -> StatKind {
        match self {
            Self::Named { .. } => StatKind::Raw,
            Self::Derived { .. } => StatKind::Derived,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum StatCmd {
    /// Print all statistics (raw counters + derived entries)
    Print,
    /// Print raw counters only
    Raw,
    /// Print IPC statistics (inst/cycle counters + derived IPC)
    Ipc,
    /// Print branch-predictor statistics (branch/mispred counters + derived rate)
    Bp,
}

impl StatCmd {
    /// The derive-rule group this subcommand focuses on, if any.
    pub fn group(&self) -> Option<&'static str> {
        match self {
            Self::Ipc => Some("ipc"),
            Self::Bp => Some("bp"),
            Self::Print | Self::Raw => None,
        }
    }
}
