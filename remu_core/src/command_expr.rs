use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;
use shlex;
use thiserror::Error;

#[derive(Parser)]
#[grammar = "command_expr.pest"]
struct ExprParser;

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
    #[error("empty command block")]
    EmptyBlock,
    #[error("invalid quoting inside block")]
    InvalidQuoting,
}

pub(crate) fn parse_expression(input: &str) -> Result<CommandExpr, ParseError> {
    let input = input.trim();
    let mut pairs =
        ExprParser::parse(Rule::expr, input).map_err(|e| ParseError::Pest(e.to_string()))?;

    let expr_pair = pairs
        .next()
        .ok_or_else(|| ParseError::Pest("missing expr".to_string()))?;

    let mut inner = expr_pair.into_inner();

    // First block
    let first_block = inner
        .next()
        .ok_or_else(|| ParseError::Pest("missing first block".into()))?;
    let first = block_to_tokens(first_block)?;

    // Zero or more (op, block)
    let mut tail = Vec::new();
    while let Some(op_pair) = inner.next() {
        let op = match op_pair.as_rule() {
            Rule::and => Op::And,
            Rule::or => Op::Or,
            Rule::EOI => break,
            _ => {
                return Err(ParseError::Pest(format!(
                    "unexpected op: {:?}",
                    op_pair.as_rule()
                )));
            }
        };

        let block_pair = inner
            .next()
            .ok_or_else(|| ParseError::Pest("operator missing right-hand block".into()))?;
        let block = block_to_tokens(block_pair)?;
        tail.push((op, block));
    }

    Ok(CommandExpr { first, tail })
}

fn block_to_tokens(block: Pair<Rule>) -> Result<Vec<String>, ParseError> {
    // block -> do_kw WHITESPACE+ "{" inner? "}"
    let mut inner = block.into_inner();
    let inner_pair = inner
        .find(|p| p.as_rule() == Rule::inner)
        .map(|p| p.as_str())
        .unwrap_or("");

    let tokens = shlex::split(inner_pair).ok_or(ParseError::InvalidQuoting)?;
    if tokens.is_empty() {
        return Err(ParseError::EmptyBlock);
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_incomplete_second_block() {
        let input = "do { continue } and do { times count help";
        let err = parse_expression(input).unwrap_err();
        assert!(matches!(err, ParseError::Pest(_)));
    }

    #[test]
    fn rejects_trailing_garbage_after_expr() {
        let input = "do { continue } and do { times count help } trailing";
        let err = parse_expression(input).unwrap_err();
        assert!(matches!(err, ParseError::Pest(_)));
    }

    #[test]
    fn accepts_valid_two_blocks() {
        let input = "do { continue } and do { times count help }";
        assert!(parse_expression(input).is_ok());
    }
}
