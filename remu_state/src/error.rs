use std::backtrace::Backtrace;
use thiserror::Error;

use crate::bus::BusError;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("bus error: {0}")]
    BusError(Box<BusError>),
}

impl From<BusError> for StateError {
    #[inline(always)]
    fn from(e: BusError) -> Self {
        StateError::BusError(Box::new(e))
    }
}

impl StateError {
    #[inline(always)]
    pub fn backtrace(&self) -> Option<&Backtrace> {
        match self {
            StateError::BusError(b) => b.backtrace(),
        }
    }

    #[inline(always)]
    pub fn exit_code(&self) -> Option<remu_types::ExitCode> {
        match self {
            StateError::BusError(b) => match b.as_ref() {
                BusError::ProgramExit(ec) => Some(*ec),
                _ => None,
            },
        }
    }
}
