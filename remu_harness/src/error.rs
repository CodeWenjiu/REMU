use remu_simulator::SimulatorError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HarnessError {
    #[error("interrupted")]
    Interrupted,

    #[error(transparent)]
    Simulator(#[from] SimulatorError),
}

impl HarnessError {
    #[inline(always)]
    pub fn backtrace(&self) -> Option<&std::backtrace::Backtrace> {
        match self {
            HarnessError::Interrupted => None,
            HarnessError::Simulator(e) => e.backtrace(),
        }
    }
}
