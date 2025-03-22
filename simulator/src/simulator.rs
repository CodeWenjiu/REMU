use std::{cell::RefCell, rc::Rc};

use logger::Logger;
use remu_utils::{Platform, ProcessResult, Simulators};
use enum_dispatch::enum_dispatch;
use state::States;

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

impl TryFrom<(&Platform, Rc<RefCell<States>>)> for SimulatorImpl {
    type Error = SimulatorError;

    fn try_from(sim_cfg: (&Platform, Rc<RefCell<States>>)) -> Result<Self, Self::Error> {
        let (sim_isa, states) = sim_cfg;
        let sim = sim_isa.simulator;
        let isa = sim_isa.isa;
        match sim {
            Simulators::EMU => Ok(SimulatorImpl::NEMU(Emu::new(isa, states))),
            Simulators::NPC => {
                Logger::show("NPC is not implemented yet", Logger::ERROR);
                Err(SimulatorError::UnknownSimulator)
            }
        }
    }
}
