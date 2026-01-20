use miette::Diagnostic;
use shlex;
use thiserror::Error;
use winnow::Parser as _;
use winnow::ascii::{multispace0, multispace1};
use winnow::combinator::{alt, cut_err, delimited, eof, opt, repeat};
use winnow::error::{ContextError, ErrMode};
use winnow::token::take_until;

/// Logical operators supported in command expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Op {
    And,
    Or,
}

/// AST for a command expression: first block plus zero or more (op, block).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandExpr {
    pub(crate) first: Vec<String>,
    pub(crate) tail: Vec<(Op, Vec<String>)>,
}

#[derive(Debug, Error, Diagnostic)]
pub enum ParseError {
    #[error("parse error: {0}")]
    Winnow(String),

    #[error("parse error (handled)")]
    WinnowHandled,

    #[error("invalid quoting inside block")]
    InvalidQuoting,
}

pub(crate) fn parse_expression(input: &str) -> Result<CommandExpr, ParseError> {
    let input = input.trim();

    if input.is_empty() {
        return Ok(CommandExpr {
            first: Vec::new(),
            tail: Vec::new(),
        });
    }

    let result: Result<CommandExpr, ParseError> = (|| {
        let mut s = input;

        // Equivalent to the pest grammar:
        // expr = SOI ~ block ~ (WS* ~ (and|or) ~ WS* ~ block)* ~ WS* ~ EOI
        let mut expr = (
            multispace0,
            parse_block,
            repeat(0.., (multispace0, parse_op, multispace0, parse_block)),
            multispace0,
            eof,
        )
            .map(
                |(_, first, tail, _, _): (_, Vec<String>, Vec<(_, Op, _, Vec<String>)>, _, _)| {
                    CommandExpr {
                        first,
                        tail: tail
                            .into_iter()
                            .map(|(_, op, _, block)| (op, block))
                            .collect(),
                    }
                },
            );

        expr.parse_next(&mut s)
            .map_err(|e| ParseError::Winnow(format!("{e:?}")))
    })();

    match result {
        Ok(expr) => Ok(expr),
        Err(e) => {
            let _ = eprintln!("{}", e);
            Err(ParseError::WinnowHandled)
        }
    }
}

fn parse_op(input: &mut &str) -> winnow::Result<Op, ErrMode<ContextError>> {
    alt((
        ("and", multispace1).map(|_| Op::And),
        ("and", eof).map(|_| Op::And),
        ("or", multispace1).map(|_| Op::Or),
        ("or", eof).map(|_| Op::Or),
    ))
    .parse_next(input)
}

fn parse_block(input: &mut &str) -> winnow::Result<Vec<String>, ErrMode<ContextError>> {
    alt((parse_brace_block, parse_command)).parse_next(input)
}

fn parse_brace_block(input: &mut &str) -> winnow::Result<Vec<String>, ErrMode<ContextError>> {
    // `{` ~ inner? ~ `}`
    //
    // We intentionally keep "inner" permissive (like the pest grammar), and
    // leave quoting validation to shlex (same behavior as before).
    let inner_str = delimited("{", opt(take_until(0.., "}")), cut_err("}"))
        .map(|opt| opt.unwrap_or(""))
        .parse_next(input)?;

    let tokens = shlex::split(inner_str).ok_or_else(|| ErrMode::Cut(ContextError::new()))?;

    Ok(tokens)
}

fn parse_command(input: &mut &str) -> winnow::Result<Vec<String>, ErrMode<ContextError>> {
    // command = token ~ (WS+ ~ token)*
    //
    // We parse the command source slice until:
    // - end of input, or
    // - a logical operator ("and"/"or") that is a separate token (followed by WS or EOI)
    //
    // Then we shlex-split that slice into tokens.
    let rest = *input;

    let mut idx = 0;
    let bytes = rest.as_bytes();

    // Track whether we're currently inside whitespace (token boundary).
    let mut prev_is_ws = true;

    while idx < bytes.len() {
        let b = bytes[idx];
        let is_ws = matches!(b, b' ' | b'\t' | b'\n' | b'\r');

        if prev_is_ws {
            if rest[idx..].starts_with("and") {
                let after = idx + 3;
                if after == bytes.len() || matches!(bytes[after], b' ' | b'\t' | b'\n' | b'\r') {
                    break;
                }
            }
            if rest[idx..].starts_with("or") {
                let after = idx + 2;
                if after == bytes.len() || matches!(bytes[after], b' ' | b'\t' | b'\n' | b'\r') {
                    break;
                }
            }
        }

        prev_is_ws = is_ws;
        idx += 1;
    }

    let cmd_src = &rest[..idx];
    let tokens = shlex::split(cmd_src).ok_or_else(|| ErrMode::Cut(ContextError::new()))?;

    // Advance input by consumed command slice.
    *input = &rest[idx..];

    Ok(tokens)
}
