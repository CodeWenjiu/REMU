use thiserror::Error;

use crate::bus::BusFault;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StateError {
    #[error("bus read error")]
    BusError(#[from] BusFault),
}
