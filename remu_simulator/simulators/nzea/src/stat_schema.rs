//! Stat schema: the RTL declares what it measures, remu reads the declaration,
//! evaluates it and renders it. No metric, region or format string is known to
//! remu any more (see the nzea handoff document, schema = 1).
//!
//! The schema file is `<verilog_dir>/<Design>.stats.toml`, emitted by nzea's
//! dump next to `filelist.f`. The expression grammar and its evaluation
//! semantics follow `nzea_rtl/src/StatExpr.scala` (the normative reference,
//! pinned by the tests below); the parser itself is built from winnow
//! combinators, as elsewhere in the repo. The one extension is that
//! unavailability carries a *reason* for display (`n/a (…)`), which the Scala
//! reference does not need.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use winnow::Parser as _;
use winnow::ascii::multispace0;
use winnow::combinator::{alt, repeat};
use winnow::error::{ErrMode, ParserError};
use winnow::stream::{LocatingSlice, Location, Stream};
use winnow::token::{one_of, take_while};

/// Schema format version this remu understands. Unknown versions fail loudly.
const SUPPORTED_SCHEMA: i64 = 1;

/// Largest counter width that fits the u64 C callback.
const MAX_COUNTER_WIDTH: i64 = 64;

/// Largest accepted `digits` (f64 has ~17 significant decimal digits).
const MAX_DIGITS: i64 = 30;

// ---------------------------------------------------------------------------
// Expression: AST, parser (winnow), evaluator — grammar per StatExpr.scala
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Func {
    Abs,
    Floor,
    Max,
    Min,
}

#[derive(Debug, Clone)]
enum Node {
    Num(f64),
    Ref(String),
    Neg(Box<Node>),
    Bin {
        op: BinOp,
        lhs: Box<Node>,
        rhs: Box<Node>,
        /// Source text of the right-hand side (used to name a zero denominator).
        rhs_text: String,
    },
    Call {
        func: Func,
        args: Vec<Node>,
    },
}

impl Node {
    fn collect_refs<'a>(&'a self, out: &mut Vec<&'a str>) {
        match self {
            Node::Num(_) => {}
            Node::Ref(name) => out.push(name),
            Node::Neg(arg) => arg.collect_refs(out),
            Node::Bin { lhs, rhs, .. } => {
                lhs.collect_refs(out);
                rhs.collect_refs(out);
            }
            Node::Call { args, .. } => {
                for a in args {
                    a.collect_refs(out);
                }
            }
        }
    }
}

/// Why a derived entry cannot be computed. Rendered as `n/a (<reason>)`, never
/// as a fabricated number.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Unavailable {
    /// A referenced counter/derived value is not available.
    Missing(String),
    /// Division whose denominator is zero; carries the denominator's source text.
    DivByZero(String),
    /// Arithmetic left the finite range (`inf`/`NaN`): the value is meaningless,
    /// so the entry is unavailable instead of a fabricated number.
    Overflow,
}

impl Unavailable {
    fn reason(&self) -> String {
        match self {
            Unavailable::Missing(name) => format!("missing: {name}"),
            Unavailable::DivByZero(text) => format!("div by zero: {text}"),
            Unavailable::Overflow => "overflow".to_string(),
        }
    }
}

/// Evaluate `node`; `lookup` returns `None` for an unavailable reference.
/// Unavailability propagates: the first unavailable operand wins.
///
/// Every result is checked for finiteness, so an expression that overflows is
/// unavailable (`n/a (overflow)`) rather than `inf`/`NaN`.
fn eval(node: &Node, lookup: &mut dyn FnMut(&str) -> Option<f64>) -> Result<f64, Unavailable> {
    match node {
        Node::Num(v) => finite(*v),
        Node::Ref(name) => lookup(name).ok_or_else(|| Unavailable::Missing(name.clone())),
        Node::Neg(arg) => finite(-eval(arg, lookup)?),
        Node::Bin {
            op,
            lhs,
            rhs,
            rhs_text,
        } => {
            let l = eval(lhs, lookup)?;
            let r = eval(rhs, lookup)?;
            let value = match op {
                BinOp::Add => l + r,
                BinOp::Sub => l - r,
                BinOp::Mul => l * r,
                BinOp::Div => {
                    if r == 0.0 {
                        return Err(Unavailable::DivByZero(rhs_text.clone()));
                    }
                    l / r
                }
            };
            finite(value)
        }
        Node::Call { func, args } => {
            let mut vals = Vec::with_capacity(args.len());
            for a in args {
                vals.push(eval(a, lookup)?);
            }
            finite(match func {
                Func::Abs => vals[0].abs(),
                Func::Floor => vals[0].floor(),
                Func::Max => vals[0].max(vals[1]),
                Func::Min => vals[0].min(vals[1]),
            })
        }
    }
}

/// Reject non-finite values: `inf`/`NaN` never reach the display layer.
fn finite(value: f64) -> Result<f64, Unavailable> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Unavailable::Overflow)
    }
}

/// Parse an expression. Fails with the same messages as the Scala reference
/// (including the offset into the source).
fn parse(src: &str) -> Result<Node, String> {
    let mut input = Expr::new(src);
    expression_all.parse_next(&mut input).map_err(|e| match e {
        ErrMode::Backtrack(e) | ErrMode::Cut(e) => e.render(src),
        // A complete `&str` never asks for more data.
        ErrMode::Incomplete(_) => ExprError {
            msg: "unexpected end of expression".to_string(),
            pos: src.len(),
        }
        .render(src),
    })
}

/// Parser input: a `&str` that tracks byte offsets, so a failure can name the
/// position it was detected at.
type Expr<'i> = LocatingSlice<&'i str>;

/// Parser result: [`ErrMode::Backtrack`] lets an enclosing `alt` try another
/// branch, [`ErrMode::Cut`] reports the failure as final.
type PResult<'i, T> = winnow::ModalResult<T, ExprError>;

/// Why an expression failed to parse, and where.
#[derive(Debug)]
struct ExprError {
    msg: String,
    /// Byte offset into the expression source.
    pos: usize,
}

impl ExprError {
    /// `{msg} (at offset {pos} of "{src}")`, as the Scala reference renders it.
    fn render(&self, src: &str) -> String {
        format!("{} (at offset {} of \"{}\")", self.msg, self.pos, src)
    }
}

impl<'i> ParserError<Expr<'i>> for ExprError {
    type Inner = Self;

    /// Fallback for failures raised by winnow's own combinators; the leaf
    /// parsers below always supply a message of their own.
    fn from_input(input: &Expr<'i>) -> Self {
        ExprError {
            msg: "invalid expression".to_string(),
            pos: offset(input),
        }
    }

    fn into_inner(self) -> Result<Self::Inner, Self> {
        Ok(self)
    }
}

/// Byte offset of the cursor in the expression source.
fn offset(input: &Expr<'_>) -> usize {
    Location::current_token_start(input)
}

/// Fail at `pos` with `msg`; `Cut` keeps an enclosing `alt` from swallowing it.
fn fail_at<'i, T>(pos: usize, msg: impl Into<String>) -> PResult<'i, T> {
    Err(ErrMode::Cut(ExprError {
        msg: msg.into(),
        pos,
    }))
}

/// Fail at the cursor (see [`fail_at`]).
fn fail<'i, T>(input: &Expr<'i>, msg: impl Into<String>) -> PResult<'i, T> {
    fail_at(offset(input), msg)
}

/// Whitespace, or nothing at all.
fn ws<'i>(input: &mut Expr<'i>) -> PResult<'i, ()> {
    let _ = multispace0.parse_next(input)?;
    Ok(())
}

/// The whole grammar: an expression, whitespace, end of input.
fn expression_all<'i>(input: &mut Expr<'i>) -> PResult<'i, Node> {
    ws.parse_next(input)?;
    let node = expression.parse_next(input)?;
    ws.parse_next(input)?;
    if input.eof_offset() == 0 {
        Ok(node)
    } else {
        fail(input, "unexpected trailing input")
    }
}

/// `expr := term (('+' | '-') term)*`, left-associative.
fn expression<'i>(input: &mut Expr<'i>) -> PResult<'i, Node> {
    let first = term.parse_next(input)?;
    repeat(0.., (ws, add_op, ws, term.with_taken()))
        .fold(
            || first.clone(),
            |lhs, (_, op, _, (rhs, rhs_text))| bin(op, lhs, rhs, rhs_text),
        )
        .parse_next(input)
}

/// `term := factor (('*' | '/') factor)*`, left-associative.
fn term<'i>(input: &mut Expr<'i>) -> PResult<'i, Node> {
    let first = factor.parse_next(input)?;
    repeat(0.., (ws, mul_op, ws, factor.with_taken()))
        .fold(
            || first.clone(),
            |lhs, (_, op, _, (rhs, rhs_text))| bin(op, lhs, rhs, rhs_text),
        )
        .parse_next(input)
}

/// A binary node; `rhs_text` is the RHS source text, used to name a zero
/// denominator.
fn bin(op: BinOp, lhs: Node, rhs: Node, rhs_text: &str) -> Node {
    Node::Bin {
        op,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
        rhs_text: rhs_text.to_string(),
    }
}

/// `factor := '-' factor | '(' expr ')' | number | reference`; anything else is
/// reported by [`unexpected`].
fn factor<'i>(input: &mut Expr<'i>) -> PResult<'i, Node> {
    alt((negative, group, number, reference, unexpected)).parse_next(input)
}

/// `'-' factor` (whitespace allowed after the sign).
fn negative<'i>(input: &mut Expr<'i>) -> PResult<'i, Node> {
    let _ = one_of('-').parse_next(input)?;
    ws.parse_next(input)?;
    Ok(Node::Neg(Box::new(factor.parse_next(input)?)))
}

/// `'(' expr ')'`; parentheses group, they are not part of the AST.
fn group<'i>(input: &mut Expr<'i>) -> PResult<'i, Node> {
    let _ = one_of('(').parse_next(input)?;
    ws.parse_next(input)?;
    let node = expression.parse_next(input)?;
    ws.parse_next(input)?;
    expect_char(input, ')')?;
    Ok(node)
}

/// `number := (digit | '.')+` — decimal, optionally fractional; no sign, no
/// exponent, no suffix.
fn number<'i>(input: &mut Expr<'i>) -> PResult<'i, Node> {
    let (text, span) = take_while(1.., |c: char| c.is_ascii_digit() || c == '.')
        .with_span()
        .parse_next(input)?;
    match text.parse::<f64>() {
        Ok(value) => Ok(Node::Num(value)),
        Err(_) => fail_at(span.start, format!("invalid number '{text}'")),
    }
}

/// `reference := ident | ident '(' expr (',' expr)* ')'` — a counter/derived
/// reference, or a function call.
fn reference<'i>(input: &mut Expr<'i>) -> PResult<'i, Node> {
    let (name, span) = take_while(1.., |c: char| c.is_alphanumeric() || c == '_')
        .with_span()
        .parse_next(input)?;
    // `max (1, 2)` is a call (whitespace may precede `(`), but a bare reference
    // must not swallow the whitespace after it: the source text of a binary RHS
    // is measured by the cursor. Probe on a copy — same offset space, discarded
    // either way.
    let mut probe = *input;
    let call: PResult<'i, ((), char)> = (ws, one_of('(')).parse_next(&mut probe);
    if call.is_err() {
        return Ok(Node::Ref(name.to_string()));
    }
    let args = call_args.parse_next(input)?;
    let func = match (name, args.len()) {
        ("abs", 1) => Func::Abs,
        ("floor", 1) => Func::Floor,
        ("max", 2) => Func::Max,
        ("min", 2) => Func::Min,
        ("abs" | "floor", n) => {
            return fail_at(
                span.start,
                format!("function '{name}' takes 1 argument(s), got {n}"),
            );
        }
        ("max" | "min", n) => {
            return fail_at(
                span.start,
                format!("function '{name}' takes 2 argument(s), got {n}"),
            );
        }
        _ => return fail_at(span.start, format!("unknown function '{name}'")),
    };
    Ok(Node::Call { func, args })
}

/// `'(' (expr (',' expr)*)? ')'` — the argument list of a function call.
fn call_args<'i>(input: &mut Expr<'i>) -> PResult<'i, Vec<Node>> {
    let _ = one_of('(').parse_next(input)?;
    ws.parse_next(input)?;
    let mut args = Vec::new();
    while input.peek_token() != Some(')') {
        args.push(expression.parse_next(input)?);
        ws.parse_next(input)?;
        if input.peek_token() != Some(',') {
            break;
        }
        let _ = input.next_token();
        ws.parse_next(input)?;
    }
    expect_char(input, ')')?;
    Ok(args)
}

/// Consume `c`; anything else fails with `expected 'c'`.
fn expect_char<'i>(input: &mut Expr<'i>, c: char) -> PResult<'i, ()> {
    if input.peek_token() == Some(c) {
        let _ = input.next_token();
        Ok(())
    } else {
        fail(input, format!("expected '{c}'"))
    }
}

/// Last `factor` alternative: name what is actually there.
fn unexpected<'i>(input: &mut Expr<'i>) -> PResult<'i, Node> {
    match input.peek_token() {
        None => fail(input, "unexpected end of expression"),
        Some(c) => fail(input, format!("unexpected character '{c}'")),
    }
}

/// `'+'` / `'-'`.
fn add_op<'i>(input: &mut Expr<'i>) -> PResult<'i, BinOp> {
    one_of(['+', '-'])
        .map(|c| if c == '+' { BinOp::Add } else { BinOp::Sub })
        .parse_next(input)
}

/// `'*'` / `'/'`.
fn mul_op<'i>(input: &mut Expr<'i>) -> PResult<'i, BinOp> {
    one_of(['*', '/'])
        .map(|c| if c == '*' { BinOp::Mul } else { BinOp::Div })
        .parse_next(input)
}

// ---------------------------------------------------------------------------
// Schema types
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(src: &str, vars: &[(&str, f64)]) -> Result<f64, Unavailable> {
        let node = parse(src).expect("parse");
        let mut lookup = |name: &str| vars.iter().find(|(n, _)| *n == name).map(|(_, v)| *v);
        eval(&node, &mut lookup)
    }

    fn ok(src: &str, vars: &[(&str, f64)]) -> f64 {
        ev(src, vars).expect("available")
    }

    #[test]
    fn precedence_and_parens() {
        assert_eq!(ok("1 + 2 * 3", &[]), 7.0);
        assert_eq!(ok("(1 + 2) * 3", &[]), 9.0);
        assert_eq!(ok("10 - 2 - 3", &[]), 5.0);
        assert_eq!(ok("-2 * 3", &[]), -6.0);
        assert_eq!(ok("2 * -3", &[]), -6.0);
        assert_eq!(ok("1 - -2", &[]), 3.0);
    }

    #[test]
    fn fractional_division_is_float() {
        assert_eq!(ok("3 / 2", &[]), 1.5);
        assert_eq!(ok("0.5 * 4", &[]), 2.0);
    }

    #[test]
    fn non_finite_results_are_unavailable() {
        // An overflowing product must not surface as `inf`.
        assert_eq!(
            ev("stat_a * stat_a", &[("stat_a", 1e300)]),
            Err(Unavailable::Overflow)
        );
        // ...and it propagates like any other unavailability (`inf - inf` would
        // have produced NaN).
        assert_eq!(
            ev(
                "stat_a * stat_a - stat_b * stat_b",
                &[("stat_a", 1e300), ("stat_b", 1e300)]
            ),
            Err(Unavailable::Overflow)
        );
        // A literal too large for f64 parses to `inf`, which is not a number
        // this schema can report either.
        assert_eq!(ev(&"9".repeat(400), &[]), Err(Unavailable::Overflow));

        let derived = Derived {
            name: "x".to_string(),
            expr: "stat_a * stat_a".to_string(),
            node: parse("stat_a * stat_a").expect("parse"),
            region: "throughput".to_string(),
            unit: "%".to_string(),
            digits: 2,
            doc: String::new(),
        };
        assert_eq!(
            derived.render(&Err(Unavailable::Overflow)),
            "n/a (overflow)"
        );
        // The renderer keeps the same promise on its own: a non-finite value
        // handed to it directly prints `n/a (overflow)`, never `inf`/`NaN`.
        assert_eq!(derived.render(&Ok(f64::INFINITY)), "n/a (overflow)");
        assert_eq!(derived.render(&Ok(f64::NAN)), "n/a (overflow)");
    }

    #[test]
    fn division_by_zero_is_unavailable_and_named() {
        assert_eq!(
            ev("stat_a / stat_b", &[("stat_a", 1.0), ("stat_b", 0.0)]),
            Err(Unavailable::DivByZero("stat_b".to_string()))
        );
        assert_eq!(
            ev(
                "stat_a / (stat_b + stat_c)",
                &[("stat_a", 1.0), ("stat_b", 0.0), ("stat_c", 0.0)]
            ),
            Err(Unavailable::DivByZero("(stat_b + stat_c)".to_string()))
        );
    }

    #[test]
    fn missing_reference_is_unavailable() {
        assert_eq!(
            ev("stat_a + stat_b", &[("stat_a", 1.0)]),
            Err(Unavailable::Missing("stat_b".to_string()))
        );
    }

    #[test]
    fn functions_and_arity() {
        assert_eq!(ok("abs(-3)", &[]), 3.0);
        assert_eq!(ok("floor(2.75)", &[]), 2.0);
        assert_eq!(ok("max(2, 7)", &[]), 7.0);
        assert_eq!(ok("min(2, 7)", &[]), 2.0);
        assert!(parse("max(1)").unwrap_err().contains("takes 2 argument(s)"));
        assert!(
            parse("abs(1, 2)")
                .unwrap_err()
                .contains("takes 1 argument(s)")
        );
        assert!(parse("sqrt(4)").unwrap_err().contains("unknown function"));
    }

    #[test]
    fn parse_errors() {
        assert!(
            parse("1 2")
                .unwrap_err()
                .contains("unexpected trailing input")
        );
        assert!(
            parse("")
                .unwrap_err()
                .contains("unexpected end of expression")
        );
        assert!(
            parse("1 +")
                .unwrap_err()
                .contains("unexpected end of expression")
        );
        assert!(parse("(1 + 2").unwrap_err().contains("expected ')'"));
        // `$` right after an operand short-circuits as trailing input (the
        // factor-level error only fires when it is the *first* character).
        assert!(
            parse("1 $ 2")
                .unwrap_err()
                .contains("unexpected trailing input")
        );
        assert!(
            parse("$1")
                .unwrap_err()
                .contains("unexpected character '$'")
        );
        assert!(parse("1..2").unwrap_err().contains("invalid number"));
    }

    #[test]
    fn parse_errors_report_offsets() {
        // `{msg} (at offset {pos} of "{src}")`; the position names the
        // offending token, or the end of input (as in the Scala reference).
        assert_eq!(
            parse("").unwrap_err(),
            "unexpected end of expression (at offset 0 of \"\")"
        );
        assert_eq!(
            parse("$1").unwrap_err(),
            "unexpected character '$' (at offset 0 of \"$1\")"
        );
        assert_eq!(
            parse("1 +").unwrap_err(),
            "unexpected end of expression (at offset 3 of \"1 +\")"
        );
        assert_eq!(
            parse("(1 + 2").unwrap_err(),
            "expected ')' (at offset 6 of \"(1 + 2\")"
        );
        assert_eq!(
            parse("1 $ 2").unwrap_err(),
            "unexpected trailing input (at offset 2 of \"1 $ 2\")"
        );
        assert_eq!(
            parse("1..2").unwrap_err(),
            "invalid number '1..2' (at offset 0 of \"1..2\")"
        );
        assert_eq!(
            parse("max(1)").unwrap_err(),
            "function 'max' takes 2 argument(s), got 1 (at offset 0 of \"max(1)\")"
        );
    }

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
