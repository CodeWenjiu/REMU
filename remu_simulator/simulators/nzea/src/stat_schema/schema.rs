//! The stats schema file (`<verilog_dir>/<Design>.stats.toml`), written by
//! nzea's dump next to `filelist.f`.
//!
//! It declares the counters (name, width, region), the regions and the derived
//! entries (expression, region, unit, digits): the design owns the metrics,
//! remu only reads, evaluates and renders them. Declaration order is display
//! order. A file that reaches this parser was validated by the generator, and
//! anything that still does not line up — an unknown version, a duplicate name,
//! an unresolvable reference, a counter the RTL does not expose — is reported
//! as an error rather than papered over.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::Unavailable;
use super::expr::{Node, eval, parse};

/// Schema format version this remu understands. Unknown versions fail loudly.
const SUPPORTED_SCHEMA: i64 = 1;

/// Largest counter width that fits the u64 C callback.
const MAX_COUNTER_WIDTH: i64 = 64;

/// Largest accepted `digits` (f64 has ~17 significant decimal digits).
const MAX_DIGITS: i64 = 30;

#[derive(Debug)]
pub(crate) struct Region {
    pub(crate) name: String,
    /// Retained for display (handoff R13); not used by the evaluation path.
    #[allow(dead_code)]
    pub(crate) doc: String,
}

#[derive(Debug)]
pub(crate) struct Counter {
    pub(crate) name: String,
    /// Declared RTL width. Cross-checked against VPI's actual width when the
    /// counters are read; a mismatch is a build mismatch (like a missing one).
    pub(crate) width: u32,
    pub(crate) region: String,
    /// Retained for display (handoff R13).
    #[allow(dead_code)]
    pub(crate) doc: String,
}

#[derive(Debug)]
pub(crate) struct Derived {
    pub(crate) name: String,
    /// Source text, kept for diagnostics.
    #[allow(dead_code)]
    pub(crate) expr: String,
    node: Node,
    pub(crate) region: String,
    pub(crate) unit: String,
    pub(crate) digits: u32,
    /// Retained for display (handoff R13).
    #[allow(dead_code)]
    pub(crate) doc: String,
}

impl Derived {
    /// Render an evaluated value: `digits` decimals followed by `unit`, or
    /// `n/a (<reason>)` when unavailable — never a fabricated number.
    /// [`eval`] already rejects non-finite results; the guard here keeps that
    /// promise local to the display path.
    pub(crate) fn render(&self, value: &Result<f64, Unavailable>) -> String {
        match value {
            Ok(v) if v.is_finite() => format!("{:.*}{}", self.digits as usize, v, self.unit),
            Ok(_) => format!("n/a ({})", Unavailable::Overflow.reason()),
            Err(u) => format!("n/a ({})", u.reason()),
        }
    }
}

#[derive(Debug)]
pub(crate) struct StatSchema {
    pub(crate) regions: Vec<Region>,
    pub(crate) counters: Vec<Counter>,
    pub(crate) derived: Vec<Derived>,
}

impl StatSchema {
    /// Load the schema for `design` from `dir` (the directory that also holds
    /// `filelist.f`). Called once per model build/load.
    ///
    /// - `Ok(None)`: no schema file — the build exposes no stats (R11).
    /// - `Err(_)`: file present but unusable (build mismatch); surfaced when
    ///   statistics are requested, not at startup.
    pub(crate) fn load(dir: &Path, design: &str) -> Result<Option<StatSchema>, String> {
        let Some(path) = find_schema_file(dir, design)? else {
            return Ok(None);
        };
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let table: toml::Table = text
            .parse()
            .map_err(|e| format!("{}: invalid TOML: {e}", path.display()))?;
        Self::from_table(&path, &table).map(Some)
    }

    fn from_table(path: &Path, table: &toml::Table) -> Result<StatSchema, String> {
        let at = |loc: &str, key: &str| {
            format!("{}: {loc}: missing or invalid key `{key}`", path.display())
        };
        let version = table
            .get("schema")
            .and_then(toml::Value::as_integer)
            .ok_or_else(|| at("top level", "schema"))?;
        if version != SUPPORTED_SCHEMA {
            return Err(format!(
                "{}: unsupported schema version {version} (this remu supports {SUPPORTED_SCHEMA})",
                path.display()
            ));
        }
        let design = table
            .get("design")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| at("top level", "design"))?
            .to_string();
        let stem = path
            .file_name()
            .and_then(|s| s.to_str())
            .and_then(|n| n.strip_suffix(".stats.toml"))
            .unwrap_or_default();
        if stem != design {
            return Err(format!(
                "{}: design \"{design}\" does not match the file name",
                path.display()
            ));
        }

        let mut regions: Vec<Region> = Vec::new();
        for (i, r) in entries(table, "region", path)?.into_iter().enumerate() {
            let loc = format!("region[{i}]");
            let name = str_key(r, "name", path, &loc)?;
            let doc = str_key(r, "doc", path, &loc)?;
            if regions.iter().any(|x| x.name == name) {
                return Err(format!(
                    "{}: {loc}: duplicate region name `{name}`",
                    path.display()
                ));
            }
            regions.push(Region { name, doc });
        }

        let mut counters: Vec<Counter> = Vec::new();
        for (i, c) in entries(table, "counter", path)?.into_iter().enumerate() {
            let loc = format!("counter[{i}]");
            let name = str_key(c, "name", path, &loc)?;
            let width = int_key(c, "width", path, &loc)?;
            let region = str_key(c, "region", path, &loc)?;
            let doc = str_key(c, "doc", path, &loc)?;
            if !name.starts_with("stat_") {
                return Err(format!(
                    "{}: {loc}: counter name `{name}` must start with `stat_`",
                    path.display()
                ));
            }
            if !(1..=MAX_COUNTER_WIDTH).contains(&width) {
                return Err(format!(
                    "{}: {loc}: counter `{name}` declares width {width}; supported range is 1..={MAX_COUNTER_WIDTH}",
                    path.display()
                ));
            }
            if !regions.iter().any(|r| r.name == region) {
                return Err(format!(
                    "{}: {loc}: counter `{name}` declares unknown region `{region}`",
                    path.display()
                ));
            }
            if counters.iter().any(|x| x.name == name) {
                return Err(format!(
                    "{}: {loc}: duplicate counter name `{name}`",
                    path.display()
                ));
            }
            counters.push(Counter {
                name,
                width: width as u32,
                region,
                doc,
            });
        }

        let mut derived: Vec<Derived> = Vec::new();
        for (i, d) in entries(table, "derived", path)?.into_iter().enumerate() {
            let loc = format!("derived[{i}]");
            let name = str_key(d, "name", path, &loc)?;
            let expr = str_key(d, "expr", path, &loc)?;
            let region = str_key(d, "region", path, &loc)?;
            let unit = str_key(d, "unit", path, &loc)?;
            let digits = int_key(d, "digits", path, &loc)?;
            let doc = str_key(d, "doc", path, &loc)?;
            if !regions.iter().any(|r| r.name == region) {
                return Err(format!(
                    "{}: {loc}: `{name}` declares unknown region `{region}`",
                    path.display()
                ));
            }
            if counters.iter().any(|c| c.name == name) || derived.iter().any(|x| x.name == name) {
                return Err(format!(
                    "{}: {loc}: duplicate entry name `{name}`",
                    path.display()
                ));
            }
            if !(0..=MAX_DIGITS).contains(&digits) {
                return Err(format!(
                    "{}: {loc}: `{name}` declares digits {digits}; supported range is 0..={MAX_DIGITS}",
                    path.display()
                ));
            }
            let node = parse(&expr)
                .map_err(|e| format!("{}: {loc}: expression `{expr}`: {e}", path.display()))?;
            // References must resolve to a declared counter or an *earlier*
            // derived entry (evaluation is in file order, so no cycles).
            let mut refs = Vec::new();
            node.collect_refs(&mut refs);
            for r in refs {
                let known =
                    counters.iter().any(|c| c.name == r) || derived.iter().any(|x| x.name == r);
                if !known {
                    return Err(format!(
                        "{}: {loc}: `{r}` is not a declared counter or earlier derived entry",
                        path.display()
                    ));
                }
            }
            derived.push(Derived {
                name,
                expr,
                node,
                region,
                unit,
                digits: digits as u32,
                doc,
            });
        }

        Ok(StatSchema {
            regions,
            counters,
            derived,
        })
    }

    pub(crate) fn counter_index(&self, name: &str) -> Option<usize> {
        self.counters.iter().position(|c| c.name == name)
    }

    pub(crate) fn derived_index(&self, name: &str) -> Option<usize> {
        self.derived.iter().position(|d| d.name == name)
    }

    pub(crate) fn has_region(&self, name: &str) -> bool {
        self.regions.iter().any(|r| r.name == name)
    }

    /// Evaluate every derived entry in file order. References only point
    /// upwards, so a single pass resolves everything (no cycle handling).
    pub(crate) fn eval_derived(&self, raw: &HashMap<String, u64>) -> Vec<Result<f64, Unavailable>> {
        let mut values: HashMap<&str, Result<f64, Unavailable>> = HashMap::new();
        for c in &self.counters {
            if let Some(v) = raw.get(&c.name) {
                values.insert(c.name.as_str(), Ok(*v as f64));
            }
        }
        let mut out = Vec::with_capacity(self.derived.len());
        for d in &self.derived {
            let res = {
                let mut lookup =
                    |name: &str| values.get(name).and_then(|r| r.as_ref().ok().copied());
                eval(&d.node, &mut lookup)
            };
            values.insert(d.name.as_str(), res.clone());
            out.push(res);
        }
        out
    }

    /// Which entries a derived entry (transitively) depends on, as index sets.
    /// The entry itself is always included in `derived_set`.
    pub(crate) fn dependency_closure(&self, root: usize) -> (Vec<bool>, Vec<bool>) {
        let mut counter_set = vec![false; self.counters.len()];
        let mut derived_set = vec![false; self.derived.len()];
        self.walk_deps(root, &mut counter_set, &mut derived_set);
        (counter_set, derived_set)
    }

    fn walk_deps(&self, idx: usize, counter_set: &mut [bool], derived_set: &mut [bool]) {
        if derived_set[idx] {
            return;
        }
        derived_set[idx] = true;
        let mut refs = Vec::new();
        self.derived[idx].node.collect_refs(&mut refs);
        for r in refs {
            if let Some(ci) = self.counter_index(r) {
                counter_set[ci] = true;
            } else if let Some(di) = self.derived_index(r) {
                self.walk_deps(di, counter_set, derived_set);
            }
        }
    }

    /// Human-readable list for error messages (`stat <unknown>`).
    pub(crate) fn available_names(&self) -> String {
        let join = |names: Vec<&str>| {
            names
                .iter()
                .map(|n| format!("`{n}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        format!(
            "counters: {}; derived: {}; regions: {}",
            join(self.counters.iter().map(|c| c.name.as_str()).collect()),
            join(self.derived.iter().map(|d| d.name.as_str()).collect()),
            join(self.regions.iter().map(|r| r.name.as_str()).collect()),
        )
    }
}

/// Locate the schema file: `<design>.stats.toml`, else the single
/// `*.stats.toml` in the directory, else nothing.
fn find_schema_file(dir: &Path, design: &str) -> Result<Option<PathBuf>, String> {
    let direct = dir.join(format!("{design}.stats.toml"));
    if direct.is_file() {
        return Ok(Some(direct));
    }
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            let is_schema = p
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".stats.toml"));
            if is_schema {
                found.push(p);
            }
        }
    }
    found.sort();
    match found.len() {
        0 => Ok(None),
        1 => Ok(Some(found.remove(0))),
        n => Err(format!(
            "{}: {n} `*.stats.toml` files found; cannot pick one",
            dir.display()
        )),
    }
}

fn entries<'a>(
    table: &'a toml::Table,
    key: &str,
    path: &Path,
) -> Result<Vec<&'a toml::Value>, String> {
    match table.get(key) {
        None => Ok(Vec::new()),
        Some(v) => v
            .as_array()
            .map(|a| a.iter().collect())
            .ok_or_else(|| format!("{}: key `{key}` must be an array of tables", path.display())),
    }
}

fn str_key(v: &toml::Value, key: &str, path: &Path, loc: &str) -> Result<String, String> {
    v.get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("{}: {loc}: missing or invalid key `{key}`", path.display()))
}

fn int_key(v: &toml::Value, key: &str, path: &Path, loc: &str) -> Result<i64, String> {
    v.get(key)
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| format!("{}: {loc}: missing or invalid key `{key}`", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// The real NzeaCore schema, as emitted by nzea's dump.
    const CORE_SCHEMA: &str = r#"
schema = 1
design = "NzeaCore"

[[region]]
name = "throughput"
doc = "Retire throughput"

[[region]]
name = "bp"
doc = "Branch prediction"

[[counter]]
name = "stat_cycle"
width = 64
region = "throughput"
doc = "core clock cycles"

[[counter]]
name = "stat_inst_commit"
width = 64
region = "throughput"
doc = "retired instructions"

[[counter]]
name = "stat_bp_branch"
width = 32
region = "bp"
doc = "resolved branches"

[[counter]]
name = "stat_bp_mispred"
width = 32
region = "bp"
doc = "branches whose prediction was wrong"

[[counter]]
name = "stat_bp_actual_taken"
width = 32
region = "bp"
doc = "branches actually taken"

[[derived]]
name = "ipc"
expr = "stat_inst_commit / stat_cycle"
region = "throughput"
unit = ""
digits = 4
doc = "instructions per cycle"

[[derived]]
name = "cpi"
expr = "1 / ipc"
region = "throughput"
unit = ""
digits = 4
doc = "cycles per instruction"

[[derived]]
name = "mispredict_rate"
expr = "stat_bp_mispred / stat_bp_branch * 100"
region = "bp"
unit = "%"
digits = 2
doc = "share of resolved branches predicted wrongly"

[[derived]]
name = "taken_rate"
expr = "stat_bp_actual_taken / stat_bp_branch * 100"
region = "bp"
unit = "%"
digits = 2
doc = "share of resolved branches actually taken"
"#;

    fn core_schema() -> StatSchema {
        let table: toml::Table = CORE_SCHEMA.parse().expect("toml");
        StatSchema::from_table(Path::new("NzeaCore.stats.toml"), &table).expect("schema")
    }

    fn raw(vars: &[(&str, u64)]) -> HashMap<String, u64> {
        vars.iter().map(|(n, v)| (n.to_string(), *v)).collect()
    }

    #[test]
    fn schema_declaration_order_is_kept() {
        let s = core_schema();
        let names: Vec<&str> = s.counters.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "stat_cycle",
                "stat_inst_commit",
                "stat_bp_branch",
                "stat_bp_mispred",
                "stat_bp_actual_taken"
            ]
        );
        let names: Vec<&str> = s.derived.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["ipc", "cpi", "mispredict_rate", "taken_rate"]);
        assert_eq!(
            s.regions
                .iter()
                .map(|r| r.name.as_str())
                .collect::<Vec<_>>(),
            ["throughput", "bp"]
        );
    }

    /// The acceptance numbers from the handoff document (§8, hello_world run).
    #[test]
    fn evaluates_acceptance_values() {
        let s = core_schema();
        let values = s.eval_derived(&raw(&[
            ("stat_cycle", 1703),
            ("stat_inst_commit", 392),
            ("stat_bp_branch", 84),
            ("stat_bp_mispred", 66),
            ("stat_bp_actual_taken", 67),
        ]));
        assert_eq!(s.derived[0].render(&values[0]), "0.2302"); // ipc
        assert_eq!(s.derived[1].render(&values[1]), "4.3444"); // cpi = 1703/392
        assert_eq!(s.derived[2].render(&values[2]), "78.57%"); // mispredict_rate
        assert_eq!(s.derived[3].render(&values[3]), "79.76%"); // taken_rate
    }

    #[test]
    fn unavailable_renders_as_n_a_with_reason() {
        let s = core_schema();
        let values = s.eval_derived(&raw(&[
            ("stat_cycle", 0),
            ("stat_inst_commit", 392),
            ("stat_bp_branch", 0),
            ("stat_bp_mispred", 66),
            ("stat_bp_actual_taken", 67),
        ]));
        assert_eq!(
            s.derived[0].render(&values[0]),
            "n/a (div by zero: stat_cycle)"
        );
        // cpi references the (unavailable) ipc entry.
        assert_eq!(s.derived[1].render(&values[1]), "n/a (missing: ipc)");
        assert_eq!(
            s.derived[2].render(&values[2]),
            "n/a (div by zero: stat_bp_branch)"
        );
        // Overflowing arithmetic renders `n/a (overflow)`; the renderer enforces
        // that on its own too, for a non-finite value handed to it directly.
        assert_eq!(
            s.derived[0].render(&Err(Unavailable::Overflow)),
            "n/a (overflow)"
        );
        assert_eq!(s.derived[0].render(&Ok(f64::INFINITY)), "n/a (overflow)");
        assert_eq!(s.derived[0].render(&Ok(f64::NAN)), "n/a (overflow)");
    }

    #[test]
    fn dependency_closure_covers_counters_and_derived() {
        let s = core_schema();
        // ipc depends on stat_inst_commit + stat_cycle.
        let (cset, dset) = s.dependency_closure(0);
        assert!(cset[0] && cset[1] && !cset[2] && !cset[3] && !cset[4]);
        assert_eq!(dset, [true, false, false, false]);
        // cpi depends on ipc, hence transitively on both throughput counters.
        let (cset, dset) = s.dependency_closure(1);
        assert!(cset[0] && cset[1] && !cset[2]);
        assert_eq!(dset, [true, true, false, false]);
    }

    #[test]
    fn schema_validation_errors() {
        let bad = |text: &str| {
            let table: toml::Table = text.parse().expect("toml");
            StatSchema::from_table(Path::new("NzeaCore.stats.toml"), &table).unwrap_err()
        };
        assert!(
            bad("schema = 2\ndesign = \"NzeaCore\"\n").contains("unsupported schema version 2")
        );
        assert!(bad("design = \"NzeaCore\"\n").contains("`schema`"));
        assert!(bad("schema = 1\ndesign = \"Other\"\n").contains("does not match the file name"));
        assert!(
            bad(r#"
schema = 1
design = "NzeaCore"
[[counter]]
name = "stat_wide"
width = 128
region = "r"
doc = "x"
[[region]]
name = "r"
doc = "x"
"#)
            .contains("supported range is 1..=64")
        );
        assert!(
            bad(r#"
schema = 1
design = "NzeaCore"
[[region]]
name = "r"
doc = "x"
[[derived]]
name = "rate"
expr = "nosuch / stat_a"
region = "r"
unit = ""
digits = 2
doc = "x"
"#)
            .contains("is not a declared counter or earlier derived entry")
        );
        assert!(
            bad(r#"
schema = 1
design = "NzeaCore"
[[region]]
name = "r"
doc = "x"
[[counter]]
name = "stat_a"
width = 32
region = "nowhere"
doc = "x"
"#)
            .contains("unknown region")
        );
        assert!(
            bad(r#"
schema = 1
design = "NzeaCore"
[[region]]
name = "r"
doc = "x"
[[derived]]
name = "rate"
expr = "1 +"
region = "r"
unit = ""
digits = 2
doc = "x"
"#)
            .contains("unexpected end of expression")
        );
    }
}
