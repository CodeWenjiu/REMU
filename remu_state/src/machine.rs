use crate::StateError;

pub trait Machine {
    fn mem_read(&self, addr: u64) -> Result<u64, StateError>;
}
