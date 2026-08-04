//! Classification of statistics entries: raw RTL counters vs derived values.

/// Where a statistics entry comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatKind {
    /// Raw counter read directly from the platform (e.g. nzea RTL VPI signal).
    Raw,
    /// Value computed from other entries (e.g. IPC, mispredict rate).
    Derived,
}
