use pest::Parser;
use pest_derive::Parser;
use pest::pratt_parser::PrattParser;
use pest::iterators::Pairs;
use logger::Logger;
use remu_macro::log_err;
use remu_utils::{ProcessError, ProcessResult};
use state::mmu::Mask;

use crate::SimpleDebugger;

#[derive(Debug)]
pub enum BinOp {
    Add,
    Subtract,
}

#[derive(Debug)]
pub enum UnaryOp {
    Deref,
}

#[derive(Debug)]
pub enum Expr {
    Num(u32),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Bin {
        lhs: Box<Expr>,
        op: BinOp,
        rhs: Box<Expr>,
    },
}

#[derive(Parser)]
#[grammar = "cmd_parser/expr_parser.pest"]
pub struct ExprParser;

lazy_static::lazy_static! {
    static ref PRATT_PARSER: PrattParser<Rule> = {
        use pest::pratt_parser::{Assoc::*, Op};
        use Rule::*;

        // Precedence is defined lowest to highest
        PrattParser::new()
            .op(Op::prefix(deref)) // Deref has the highest precedence
            // Addition and subtract have equal precedence
            .op(Op::infix(add, Left) | Op::infix(subtract, Left))
    };
}

impl SimpleDebugger {

    fn parse_expr(pairs: Pairs<Rule>) -> Expr {
        PRATT_PARSER
            .map_primary(|primary| match primary.as_rule() {
                Rule::num => Expr::Num(primary.as_str().parse::<u32>().unwrap()),
                Rule::expr => Self::parse_expr(primary.into_inner()),
                rule => unreachable!("Expr::parse expected atom, found {:?}", rule),
            })
            .map_prefix(|op, expr| {
                let op = match op.as_rule() {
                    Rule::deref => UnaryOp::Deref,
                    rule => unreachable!("Expr::parse expected prefix operation, found {:?}", rule),
                };
                Expr::Unary {
                    op,
                    expr: Box::new(expr),
                }
            })
            .map_infix(|lhs, op, rhs| {
                let op = match op.as_rule() {
                    Rule::add => BinOp::Add,
                    Rule::subtract => BinOp::Subtract,
                    rule => unreachable!("Expr::parse expected infix operation, found {:?}", rule),
                };
                Expr::Bin {
                    lhs: Box::new(lhs),
                    op,
                    rhs: Box::new(rhs),
                }
            })
            .parse(pairs)
    }

    fn calculate_expr(&mut self, expr: &Expr) -> ProcessResult<u32> {
        match expr {
            Expr::Num(n) => Ok(*n),
            Expr::Unary { op, expr } => {
                let val = self.calculate_expr(expr)?;
                match op {
                    UnaryOp::Deref => log_err!(self.state.mmu.read_memory(val, Mask::Word), ProcessError::Recoverable),
                }
            },
            Expr::Bin { lhs, op, rhs } => {
                let lhs_val = self.calculate_expr(lhs)?;
                let rhs_val = self.calculate_expr(rhs)?;
                match op {
                    BinOp::Add => Ok(lhs_val + rhs_val),
                    BinOp::Subtract => Ok(lhs_val - rhs_val),
                }
            }
        }
    }

    pub fn eval_expr(&mut self, src: &str) -> ProcessResult<u32> {
        self.calculate_expr(&Self::parse_expr(log_err!(ExprParser::parse(Rule::expr, src), ProcessError::Recoverable)?))
    }

}