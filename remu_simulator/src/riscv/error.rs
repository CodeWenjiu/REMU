use thiserror::Error;

#[derive(Debug, Error)]
pub enum SimulatorError {
    #[error("Memory access error {0}")]
    MemoryAccessError(#[from] remu_state::bus::BusFault),
}
