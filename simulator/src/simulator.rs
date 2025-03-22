use logger::Logger;
use remu_utils::{Platform, ProcessResult, Simulators};
use enum_dispatch::enum_dispatch;

use crate::emu::Emu;

#[enum_dispatch]
pub trait Simulator {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        Logger::todo();
        Ok(())
    }
}

#[enum_dispatch(Simulator)]
pub enum SimulatorImpl {
    NEMU(Emu),
}

#[derive(Debug, snafu::Snafu)]
pub enum SimulatorError {
    #[snafu(display("Unknown Simulator"))]
    UnknownSimulator,
}

impl TryFrom<&Platform> for SimulatorImpl {
    type Error = SimulatorError;

    fn try_from(sim_isa: &Platform) -> Result<Self, Self::Error> {
        let sim = sim_isa.simulator;
        let isa = sim_isa.isa;
        match sim {
            Simulators::EMU => Ok(SimulatorImpl::NEMU(Emu::new(isa))),
            Simulators::NPC => {
                Logger::show("NPC is not implemented yet", Logger::ERROR);
                Err(SimulatorError::UnknownSimulator)
            }
        }
    }
}
