//! Derived statistics for nzea RTL `stat_*` VPI counters.
//!
//! Raw counters are enumerated dynamically (see `nzea_iter_stats`); this table
//! encodes the *semantics* of specific counters: which signals a derived entry
//! depends on, and how to compute it. `group` names the focus view exposed as
//! the `stat <group>` command (e.g. "ipc", "bp") — add a rule with a new group
//! and the command becomes available without remu changes.

/// One derived statistic: depends on several raw `stat_*` signals by leaf name.
pub(crate) struct StatDeriveRule {
    /// Focus group this entry belongs to (exposed as `stat <group>`).
    pub group: &'static str,
    /// Derived entry name (e.g. `ipc`).
    pub name: &'static str,
    /// Raw `stat_*` signal leaf names this rule consumes, in order.
    pub deps: &'static [&'static str],
    /// Compute the display value from the dependency values (same order as `deps`).
    pub derive: fn(&[u64]) -> String,
}

pub(crate) const NZEA_DERIVE_RULES: &[StatDeriveRule] = &[
    StatDeriveRule {
        group: "ipc",
        name: "ipc",
        deps: &["stat_inst_commit", "stat_cycle"],
        derive: |v| {
            let (inst, cycle) = (v[0], v[1]);
            let ipc = if cycle > 0 {
                inst as f64 / cycle as f64
            } else {
                0.0
            };
            format!("{ipc:.4}")
        },
    },
    StatDeriveRule {
        group: "bp",
        name: "mispredict_rate",
        deps: &["stat_bp_branch", "stat_bp_mispred"],
        derive: |v| {
            let (branch, mispred) = (v[0], v[1]);
            let pct = if branch > 0 {
                mispred as f64 / branch as f64 * 100.0
            } else {
                0.0
            };
            format!("{mispred}/{branch} = {pct:.2}%")
        },
    },
];
