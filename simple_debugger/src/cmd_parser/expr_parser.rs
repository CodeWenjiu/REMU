use pest::Parser;
use pest_derive::Parser;
use pest::pratt_parser::PrattParser;
use pest::iterators::Pairs;
use logger::Logger;
use remu_macro::{log_err, log_error};
use remu_utils::{ProcessError, ProcessResult};
use state::{mmu::Mask, reg::RegfileIo};

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
pub enum NameUnaryOp {
    Reg,
}

#[derive(Debug)]
pub enum Expr {
    Val(u32),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Bin {
        lhs: Box<Expr>,
        op: BinOp,
        rhs: Box<Expr>,
    },

    NameExpr(NameExpr),
}

#[derive(Debug)]
pub enum NameExpr {
    Name (String),

    NameUnary {
        op: NameUnaryOp,
        expr: Box<NameExpr>,
    }
}

#[derive(Parser)]
#[grammar = "cmd_parser/expr_parser.pest"]
pub struct ExprParser;

lazy_static::lazy_static! {
    static ref Val_PRATT_PARSER: PrattParser<Rule> = {
        use pest::pratt_parser::{Assoc::*, Op};
        use Rule::*;

        PrattParser::new()
            .op(Op::prefix(deref))
            .op(Op::infix(add, Left) | Op::infix(subtract, Left))
    };

    static ref Name_PRATT_PARSER: PrattParser<Rule> = {
        use pest::pratt_parser::Op;
        use Rule::*;

        // Precedence is defined lowest to highest
        PrattParser::new()
            .op(Op::prefix(reg))
    };
}

fn name_parse_expr(pairs: Pairs<Rule>) -> NameExpr {
    Name_PRATT_PARSER
        .map_primary(|primary| match primary.as_rule() {
            Rule::name => {
                NameExpr::Name(primary.as_str().to_string())
            }
            rule => unreachable!("Expr::parse expected atom, found {:?}", rule),
        })
        .map_prefix(|op, expr| {
            match op.as_rule() {
                Rule::reg => match expr {
                    NameExpr::Name(s) => NameExpr::NameUnary {
                        op: NameUnaryOp::Reg,
                        expr: Box::new(NameExpr::Name(s.to_string())),
                    },
                    _ => unreachable!("Reg operator expects a value operand"),
                },
                rule => unreachable!("Expr::parse expected prefix operation, found {:?}", rule),
            }
        })
        .parse(pairs)
}

impl SimpleDebugger {

    fn val_parse_expr(&mut self, pairs: Pairs<Rule>) -> ProcessResult<Expr> {
        Ok(Val_PRATT_PARSER
            .map_primary(|primary| match primary.as_rule() {
                Rule::oct => Ok(Expr::Val(primary.as_str().parse::<u32>().map_err(|_| {
                    log_error!(format!("Invalid octal value: {}", primary.as_str()));
                    ProcessError::Recoverable
                })?)),
                Rule::hex => Ok(Expr::Val(u32::from_str_radix(primary.as_str(), 16).map_err(|_| {
                    log_error!(format!("Invalid hexadecimal value: {}", primary.as_str()));
                    ProcessError::Recoverable
                })?)),
                Rule::name_term => Ok(Expr::NameExpr(name_parse_expr(primary.into_inner()))),
                Rule::expr => self.val_parse_expr(primary.into_inner()),
                rule => unreachable!("Expr::parse expected atom, found {:?}", rule),
            })
            .map_prefix(|op, expr| {
                match op.as_rule() {
                    Rule::deref => Ok(Expr::Unary {
                        op: UnaryOp::Deref,
                        expr: Box::new(expr?),
                    }),
                    rule => unreachable!("Expr::parse expected prefix operation, found {:?}", rule),
                }
            })
            .map_infix(|lhs, op, rhs| {
                let op = match op.as_rule() {
                    Rule::add => BinOp::Add,
                    Rule::subtract => BinOp::Subtract,
                    rule => unreachable!("Expr::parse expected infix operation, found {:?}", rule),
                };
                Ok(Expr::Bin {
                    lhs: Box::new(lhs?),
                    op,
                    rhs: Box::new(rhs?),
                })
            })
            .parse(pairs)?)
    }

    fn calculate_name_expr(&mut self, expr: &NameExpr) -> ProcessResult<u32> {
        match expr {
            NameExpr::Name(s) => {
                log_error!(format!("An single name {} can not evaluate to a value", s));
                Err(ProcessError::Recoverable)
            }

            NameExpr::NameUnary { op, expr } => {
                match op {
                    NameUnaryOp::Reg => {
                        match &**expr {
                            NameExpr::Name(s) => {
                                self.state.regfile.read_reg(s)
                            }

                            rule => unreachable!("Expr::parse expected a name, found {:?}", rule),
                            // NameExpr::NameUnary { op, expr } => {
                            //     log_error!("recursive evaluation of variable expressions is not supported for now");
                            //     Err(ProcessError::Recoverable)
                            // }
                        }
                    }
                }
            }
        }
    }

    fn calculate_expr(&mut self, expr: &Expr) -> ProcessResult<u32> {
        match expr {
            Expr::Val(n) => Ok(*n),
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
                    BinOp::Add => Ok(lhs_val.wrapping_add(rhs_val)),
                    BinOp::Subtract => Ok(lhs_val.wrapping_sub(rhs_val)),
                }
            },
            Expr::NameExpr(expr) => {
                self.calculate_name_expr(expr)
            }
        }
    }

    pub fn eval_expr(&mut self, src: &str) -> ProcessResult<u32> {
        let pairs = log_err!(ExprParser::parse(Rule::expr, src), ProcessError::Recoverable)?;
        let exprs = self.val_parse_expr(pairs)?;
        self.calculate_expr(&exprs)
    }

}