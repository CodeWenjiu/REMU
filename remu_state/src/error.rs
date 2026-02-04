use thiserror::Error;

use crate::bus::BusFault;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StateError {
    #[error("bus error: {0}")]
    BusError(#[from] BusFault),
}
