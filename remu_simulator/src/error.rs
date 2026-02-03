use std::fmt;

use remu_state::StateError;
use remu_types::DifftestMismatchItem;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct DifftestMismatchList(pub Vec<DifftestMismatchItem>);

impl fmt::Display for DifftestMismatchList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for item in &self.0 {
            writeln!(f, "{}", item)?;
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum SimulatorError {
    #[error("State access error {0}")]
    StateAccessError(#[from] StateError),

    #[error("Difftest mismatch: ref and DUT register state differ:\n{0}")]
    DifftestMismatch(DifftestMismatchList),

    #[error("Reference simulator error: {0}")]
    RefError(String),
}
