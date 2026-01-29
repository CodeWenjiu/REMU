use remu_state::StateError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SimulatorError {
    #[error("State access error {0}")]
    StateAccessError(#[from] StateError),
}
