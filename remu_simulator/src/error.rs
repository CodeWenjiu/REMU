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

#[derive(Debug, Error, Clone)]
pub enum RefErrorKind {
    #[error("spike difftest not initialized")]
    NotInitialized,
    #[error("(get_spike_fns().step) error: {0}")]
    StepFailed(i32),
    #[error("(get_spike_fns().get_)*_ptr returned null")]
    NullPtr,
    #[error("(get_spike_fns().write_mem) failed: addr={addr:#x}")]
    WriteMemFailed { addr: u64 },
}

#[derive(Debug, Error, Clone)]
pub enum BreakpointErrorKind {
    #[error("breakpoint address must be 4-byte aligned")]
    NotAligned,
    #[error("breakpoint at 0x{0:08x} not found")]
    NotFound(u32),
}

#[derive(Debug, Error)]
pub enum SimulatorInnerError {
    #[error("State access error {0}")]
    StateAccessError(#[from] StateError),

    #[error("Reference simulator error: {0}")]
    RefError(RefErrorKind),

    #[error("program exit: {0}")]
    ProgramExit(ExitCode),

    #[error("interrupted")]
    Interrupted,

    #[error("breakpoint: {0}")]
    BreakpointError(BreakpointErrorKind),

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
