use thiserror::Error;

use crate::bus::BusError;

#[derive(Debug, Error, Clone)]
pub enum StateError {
    #[error("bus error: {0}")]
    BusError(#[from] BusError),
}

impl StateError {
    #[inline(always)]
    pub fn backtrace(&self) -> Option<&str> {
        match self {
            StateError::BusError(b) => Some(b.backtrace()),
        }
    }
}
