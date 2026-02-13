//! Program exit semantics: GOOD (code 0) vs BAD (non-zero).

use std::fmt;

/// Exit outcome of a program (e.g. from ecall).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// Success.
    Good,
    /// Failure.
    Bad,
}

impl fmt::Display for ExitCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExitCode::Good => write!(f, "good"),
            ExitCode::Bad => write!(f, "bad"),
        }
    }
}
