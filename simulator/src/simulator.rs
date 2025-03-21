use logger::Logger;
use remu_utils::ProcessResult;
use enum_dispatch::enum_dispatch;

use crate::nemu::Nemu;

#[enum_dispatch]
pub trait Simulator {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        Logger::todo();
        Ok(())
    }
}

#[enum_dispatch(Simulator)]
pub enum SimulatorImpl {
    NEMU(Nemu),
}
