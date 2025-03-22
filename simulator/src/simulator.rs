use std::{cell::RefCell, rc::Rc};

use logger::Logger;
use remu_utils::{Disassembler, Platform, ProcessResult, Simulators};
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

impl TryFrom<(&Platform, Rc<RefCell<States>>, Rc<RefCell<Disassembler>>)> for SimulatorImpl {
    type Error = SimulatorError;

    fn try_from(sim_cfg: (&Platform, Rc<RefCell<States>>, Rc<RefCell<Disassembler>>)) -> Result<Self, Self::Error> {
        let (sim_isa, states, disasm) = sim_cfg;
        let sim = sim_isa.simulator;
        let isa = sim_isa.isa;
        match sim {
            Simulators::EMU => Ok(SimulatorImpl::NEMU(Emu::new(isa, states, disasm))),
            Simulators::NPC => {
                Logger::show("NPC is not implemented yet", Logger::ERROR);
                Err(SimulatorError::UnknownSimulator)
            }
        }
    }
}
