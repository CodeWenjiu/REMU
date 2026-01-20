use miette::Diagnostic;
use thiserror::Error;

use crate::compound_command::ParseError;

#[derive(Error, Debug, Diagnostic)]
pub enum DebuggerError {
    #[error("Command expression parse error: {0}")]
    #[diagnostic(transparent)]
    CommandExpr(#[from] ParseError),

    #[error("Command expression parse error (handled)")]
    CommandExprHandled,
}

pub(crate) type Result<T> = std::result::Result<T, DebuggerError>;
