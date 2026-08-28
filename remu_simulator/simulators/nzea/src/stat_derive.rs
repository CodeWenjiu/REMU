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
            format!("{pct:.2}%")
        },
    },
    // ── 方向 vs 目标误预测分解 ──
    StatDeriveRule {
        group: "bp",
        name: "dir_mispred_rate",
        deps: &["stat_bp_branch", "stat_bp_dir_mispred"],
        derive: |v| {
            let (branch, dir) = (v[0], v[1]);
            let pct = if branch > 0 {
                dir as f64 / branch as f64 * 100.0
            } else {
                0.0
            };
            format!("{pct:.2}%")
        },
    },
    StatDeriveRule {
        group: "bp",
        name: "tgt_mispred_rate",
        deps: &["stat_bp_branch", "stat_bp_mispred", "stat_bp_dir_mispred"],
        derive: |v| {
            let (branch, mispred, dir) = (v[0], v[1], v[2]);
            let tgt = mispred.saturating_sub(dir);
            let pct = if branch > 0 {
                tgt as f64 / branch as f64 * 100.0
            } else {
                0.0
            };
            format!("{pct:.2}%")
        },
    },
    // ── 预测利用率与负载特性 ──
    StatDeriveRule {
        group: "bp",
        name: "pred_taken_rate",
        deps: &["stat_bp_branch", "stat_bp_pred_taken"],
        derive: |v| {
            let (branch, pred) = (v[0], v[1]);
            let pct = if branch > 0 {
                pred as f64 / branch as f64 * 100.0
            } else {
                0.0
            };
            format!("{pct:.2}%")
        },
    },
    StatDeriveRule {
        group: "bp",
        name: "taken_rate",
        deps: &["stat_bp_branch", "stat_bp_actual_taken"],
        derive: |v| {
            let (branch, taken) = (v[0], v[1]);
            let pct = if branch > 0 {
                taken as f64 / branch as f64 * 100.0
            } else {
                0.0
            };
            format!("{pct:.2}%")
        },
    },
    // ── RAS（ret 预测）健康度 ──
    StatDeriveRule {
        group: "bp",
        name: "ret_mispred_rate",
        deps: &["stat_bp_ret", "stat_bp_ret_mispred"],
        derive: |v| {
            let (ret, ret_mispred) = (v[0], v[1]);
            let pct = if ret > 0 {
                ret_mispred as f64 / ret as f64 * 100.0
            } else {
                0.0
            };
            format!("{pct:.2}%")
        },
    },
    StatDeriveRule {
        group: "bp",
        name: "ret_rate",
        deps: &["stat_bp_branch", "stat_bp_ret"],
        derive: |v| {
            let (branch, ret) = (v[0], v[1]);
            let pct = if branch > 0 {
                ret as f64 / branch as f64 * 100.0
            } else {
                0.0
            };
            format!("{pct:.2}%")
        },
    },
];
