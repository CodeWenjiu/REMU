use miette::Diagnostic;
use remu_harness::SimulatorError;
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
    CommandExec(SimulatorError),

    #[error("exit requested (run state EXIT)")]
    ExitRequested,

    #[error("program exit with code {0}")]
    ProgramExit(u32),
}

impl DebuggerError {
    #[inline(always)]
    pub fn backtrace(&self) -> Option<&std::backtrace::Backtrace> {
        match self {
            DebuggerError::CommandExec(sim) => sim.backtrace(),
            _ => None,
        }
    }
}
