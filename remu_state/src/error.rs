use std::backtrace::Backtrace;
use thiserror::Error;

use crate::bus::BusError;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("bus error: {0}")]
    BusError(#[from] BusError),
}

impl StateError {
    #[inline(always)]
    pub fn backtrace(&self) -> Option<&Backtrace> {
        match self {
            StateError::BusError(b) => b.backtrace(),
        }
    }

    #[inline(always)]
    pub fn program_exit_code(&self) -> Option<u32> {
        match self {
            StateError::BusError(BusError::ProgramExit(code)) => Some(*code),
            _ => None,
        }
    }
}
