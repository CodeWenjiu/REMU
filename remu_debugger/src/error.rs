use miette::Diagnostic;
use remu_simulator::riscv::SimulatorError;
use thiserror::Error;

use crate::compound_command::ParseError;

#[derive(Error, Debug, Diagnostic)]
pub enum DebuggerError {
    #[error("Command expression parse error: {0}")]
    #[diagnostic(transparent)]
    CommandExpr(#[from] ParseError),

    #[error("Command expression parse error (handled)")]
    CommandExprHandled,

    #[error("Command execution error")]
    CommandExec(#[from] SimulatorError),
}

pub(crate) type Result<T> = std::result::Result<T, DebuggerError>;
