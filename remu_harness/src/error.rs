use remu_simulator::{SimulatorError, SimulatorInnerError};
use thiserror::Error;

/// How an error should be displayed to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorStyle {
    /// One-liner: the error message itself is sufficient.
    Trap,
    /// Full diagnostic: print error chain + backtrace for debugging.
    Diagnostic,
}

#[derive(Debug, Error)]
pub enum HarnessError {
    #[error("interrupted")]
    Interrupted,

    #[error(transparent)]
    Simulator(#[from] SimulatorError),
}

impl HarnessError {
    #[inline(always)]
    pub fn backtrace(&self) -> Option<&std::backtrace::Backtrace> {
        match self {
            HarnessError::Interrupted => None,
            HarnessError::Simulator(e) => e.backtrace(),
        }
    }

    /// Decide display style based on error kind — state access failures and
    /// difftest mismatches need full diagnostics; traps and exits are one-liners.
    pub fn style(&self) -> ErrorStyle {
        match self {
            HarnessError::Interrupted => ErrorStyle::Trap,
            HarnessError::Simulator(e) => match e {
                SimulatorError::Dut(inner) | SimulatorError::Ref(inner) => match inner {
                    SimulatorInnerError::StateAccessError(_)
                    | SimulatorInnerError::RefError(_)
                    | SimulatorInnerError::BreakpointError(_) => ErrorStyle::Diagnostic,
                    SimulatorInnerError::ProgramExit(_)
                    | SimulatorInnerError::Interrupted
                    | SimulatorInnerError::BreakpointHit(_)
                    | SimulatorInnerError::IllegalInstruction { .. } => ErrorStyle::Trap,
                },
                SimulatorError::Difftest(_) => ErrorStyle::Diagnostic,
            },
        }
    }
}
