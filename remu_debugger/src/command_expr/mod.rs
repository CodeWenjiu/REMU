use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;
use shlex;
use thiserror::Error;

#[derive(Parser)]
#[grammar = "command_expr/parse.pest"]
pub struct ExprParser;

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

#[derive(Debug, Error)]
pub(crate) enum ParseError {
    #[error("parse error: {0}")]
    Pest(String),
    #[error("parse error (handled)")]
    PestHandled,
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
        let mut pairs =
            ExprParser::parse(Rule::expr, input).map_err(|e| ParseError::Pest(e.to_string()))?;

        let expr_pair = pairs.next().unwrap();

        let mut inner = expr_pair.into_inner();

        // First block
        let first_block = inner.next().unwrap();
        let first = block_to_tokens(first_block)?;

        // Zero or more (op, block)
        let mut tail = Vec::new();
        while let Some(op_pair) = inner.next() {
            let op = match op_pair.as_rule() {
                Rule::and => Op::And,
                Rule::or => Op::Or,
                Rule::EOI => break,
                _ => {
                    unreachable!()
                }
            };

            let block_pair = inner.next().unwrap();
            let block = block_to_tokens(block_pair)?;
            tail.push((op, block));
        }

        Ok(CommandExpr { first, tail })
    })();

    match result {
        Ok(expr) => Ok(expr),
        Err(e) => {
            let _ = eprintln!("{}", e);
            Err(ParseError::PestHandled)
        }
    }
}

fn block_to_tokens(block: Pair<Rule>) -> Result<Vec<String>, ParseError> {
    match block.as_rule() {
        // Unwrap the grammar-level block wrapper
        Rule::block => {
            let mut inner = block.into_inner();
            let inner_block = inner
                .next()
                .ok_or_else(|| ParseError::Pest("empty block wrapper".to_string()))?;
            block_to_tokens(inner_block)
        }
        Rule::do_block => {
            let mut inner = block.into_inner();
            let inner_pair = inner
                .find(|p| p.as_rule() == Rule::inner)
                .map(|p| p.as_str())
                .unwrap_or("");
            let tokens = shlex::split(inner_pair).ok_or(ParseError::InvalidQuoting)?;
            if tokens.is_empty() {
                return Ok(Vec::new());
            }
            Ok(tokens)
        }
        Rule::command => {
            let src = block.as_str();
            let tokens = shlex::split(src).ok_or(ParseError::InvalidQuoting)?;
            if tokens.is_empty() {
                return Ok(Vec::new());
            }
            Ok(tokens)
        }
        other => Err(ParseError::Pest(format!(
            "unexpected block rule: {:?}",
            other
        ))),
    }
}
