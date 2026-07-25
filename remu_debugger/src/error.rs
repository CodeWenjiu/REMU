use miette::Diagnostic;
use remu_harness::HarnessError;
use remu_types::ExitCode;
use thiserror::Error;

use crate::ParseError;

#[derive(Error, Debug, Diagnostic)]
pub enum DebuggerError {
    #[error("Command expression parse error: {0}")]
    #[diagnostic(transparent)]
    CommandExpr(#[from] ParseError),

    #[error("Command expression parse error (handled)")]
    CommandExprHandled,

    #[error("Command execution error: {0}")]
    CommandExec(HarnessError),

    #[error("exit requested (run state EXIT)")]
    ExitRequested,

    #[error("program exit: {0}")]
    ProgramExit(ExitCode),
}

impl DebuggerError {
    #[inline(always)]
    pub fn backtrace(&self) -> Option<&std::backtrace::Backtrace> {
        match self {
            DebuggerError::CommandExec(harness) => harness.backtrace(),
            _ => None,
        }
    }

    /// Forward display style decision to the harness layer.
    pub fn style(&self) -> Option<remu_harness::ErrorStyle> {
        match self {
            DebuggerError::CommandExec(harness) => Some(harness.style()),
            _ => None,
        }
    }
}
