//! Unified statistics interface: platform-declared counters + derived values.
//!
//! The platform owns what it measures, how derived values are computed and how
//! they are ordered/classified; the layers above only render what it returns.
//! For nzea that declaration is the RTL's stats schema (`*.stats.toml`).

use remu_types::StatKind;

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

/// The `stat` command: what to select out of the platform's declaration. Like
/// [`StateCmd`](remu_state::StateCmd), it is handed to the platform as-is; the
/// platform owns filtering and ordering.
#[derive(Debug, Clone)]
pub enum StatCmd {
    /// Everything: every counter, then every derived entry, declaration order
    /// (default, also spelled `stat print`).
    All,
    /// Raw counters only, declaration order.
    Raw,
    /// A query against the platform's declaration: an exact counter or derived
    /// entry name, else a region name (resolution order: entry, then region).
    Query(String),
}

impl StatCmd {
    /// Parse the `stat` argument: `None`/`print` → everything, `raw` → counters
    /// only, anything else → a query resolved against the platform's declaration.
    pub fn from_query(query: Option<&str>) -> Self {
        match query {
            None | Some("print") => StatCmd::All,
            Some("raw") => StatCmd::Raw,
            Some(q) => StatCmd::Query(q.to_string()),
        }
    }
}
