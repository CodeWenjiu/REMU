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
pub enum SimulatorInnerError {
    #[error("State access error {0}")]
    StateAccessError(#[from] StateError),

    #[error("Reference simulator error: {0}")]
    RefError(String),
}

impl SimulatorInnerError {
    #[inline(always)]
    pub fn backtrace(&self) -> Option<&str> {
        match self {
            SimulatorInnerError::StateAccessError(e) => e.backtrace(),
            SimulatorInnerError::RefError(_) => None,
        }
    }
}

#[derive(Debug, Error)]
pub enum SimulatorError {
    #[error("Ref error: {0}")]
    Ref(SimulatorInnerError),

    #[error("Dut error: {0}")]
    Dut(SimulatorInnerError),

    #[error("Difftest mismatch: ref and DUT register state differ:\n{0}")]
    Difftest(DifftestMismatchList),
}

impl SimulatorError {
    #[inline(always)]
    pub fn backtrace(&self) -> Option<&str> {
        match self {
            SimulatorError::Dut(e) | SimulatorError::Ref(e) => e.backtrace(),
            SimulatorError::Difftest(_) => None,
        }
    }
}
