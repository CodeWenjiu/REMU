use std::fmt;

use remu_state::StateError;
use remu_types::{DifftestMismatchItem, ExitCode};
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

    #[error("program exit: {0}")]
    ProgramExit(ExitCode),

    #[error("interrupted")]
    Interrupted,

    #[error("breakpoint: {0}")]
    BreakpointError(String),

    /// DUT hit a breakpoint (ebreak at this PC). Execution stopped.
    #[error("breakpoint hit at 0x{0:08x}")]
    BreakpointHit(u32),
}

impl SimulatorInnerError {
    #[inline(always)]
    pub fn backtrace(&self) -> Option<&std::backtrace::Backtrace> {
        match self {
            SimulatorInnerError::StateAccessError(e) => e.backtrace(),
            SimulatorInnerError::RefError(_)
            | SimulatorInnerError::ProgramExit(_)
            | SimulatorInnerError::Interrupted
            | SimulatorInnerError::BreakpointError(_)
            | SimulatorInnerError::BreakpointHit(_) => None,
        }
    }
}

pub fn from_state_error(e: StateError) -> SimulatorInnerError {
    if let Some(exit_code) = e.exit_code() {
        SimulatorInnerError::ProgramExit(exit_code)
    } else if let Some(pc) = e.breakpoint_pc() {
        SimulatorInnerError::BreakpointHit(pc)
    } else {
        SimulatorInnerError::StateAccessError(e)
    }
}

#[derive(Debug, Error)]
pub enum SimulatorError {
    #[error("Ref error: {0}")]
    Ref(SimulatorInnerError),

    #[error("Dut error: {0}")]
    Dut(SimulatorInnerError),

    #[error("Difftest mismatch: ref and DUT state differ:\n{0}")]
    Difftest(DifftestMismatchList),
}

impl SimulatorError {
    #[inline(always)]
    pub fn backtrace(&self) -> Option<&std::backtrace::Backtrace> {
        match self {
            SimulatorError::Dut(e) | SimulatorError::Ref(e) => e.backtrace(),
            SimulatorError::Difftest(_) => None,
        }
    }
}
