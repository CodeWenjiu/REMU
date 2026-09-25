//! Expression language of the stats schema: AST, winnow parser, evaluator.
//!
//! Grammar (see `nzea_rtl/src/StatExpr.scala`, the normative reference):
//!
//! ```text
//! expr   := term (('+' | '-') term)*
//! term   := factor (('*' | '/') factor)*
//! factor := '-' factor | '(' expr ')' | number | reference
//! ```
//!
//! `number` is decimal and optionally fractional; `reference` is a counter
//! name, a derived name declared above it, or a call to `abs`/`floor`/`max`/
//! `min` — nothing else, no conditionals and no host hooks. Entries are
//! evaluated in file order, so references only point upwards and no cycle
//! handling is needed. Unavailability — a missing reference, a zero
//! denominator, an overflow — propagates instead of yielding a fabricated
//! value, and carries a reason so the display layer can render `n/a (…)`.

use winnow::Parser as _;
use winnow::ascii::multispace0;
use winnow::combinator::{alt, repeat};
use winnow::error::{ErrMode, ParserError};
use winnow::stream::{LocatingSlice, Location, Stream};
use winnow::token::{one_of, take_while};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Func {
    Abs,
    Floor,
    Max,
    Min,
}

#[derive(Debug, Clone)]
pub(super) enum Node {
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
    pub(super) fn collect_refs<'a>(&'a self, out: &mut Vec<&'a str>) {
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
    pub(super) fn reason(&self) -> String {
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
pub(super) fn eval(
    node: &Node,
    lookup: &mut dyn FnMut(&str) -> Option<f64>,
) -> Result<f64, Unavailable> {
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
pub(super) fn parse(src: &str) -> Result<Node, String> {
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
}
