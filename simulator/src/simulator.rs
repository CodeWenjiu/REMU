use std::{cell::RefCell, rc::Rc};

use logger::Logger;
use option_parser::OptionParser;
use remu_utils::{Disassembler, ProcessResult, Simulators};
use enum_dispatch::enum_dispatch;
use state::States;

use crate::emu::Emu;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum FunctionTarget {
    /// The instruction trace function
    InstructionTrace,
}

#[enum_dispatch]
pub trait Simulator {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        Logger::todo();
        Ok(())
    }

    fn cmd_function_mut (&mut self, _target: FunctionTarget, _enable: bool) -> ProcessResult<()> {
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

impl TryFrom<(&OptionParser, Rc<RefCell<States>>, Rc<RefCell<Disassembler>>)> for SimulatorImpl {
    type Error = SimulatorError;

    fn try_from(sim_cfg: (&OptionParser, Rc<RefCell<States>>, Rc<RefCell<Disassembler>>)) -> Result<Self, Self::Error> {
        let (option, states, disasm) = sim_cfg;
        let sim = option.cli.platform.simulator;
        match sim {
            Simulators::EMU => Ok(SimulatorImpl::NEMU(Emu::new(option, states, disasm))),
            Simulators::NPC => {
                Logger::show("NPC is not implemented yet", Logger::ERROR);
                Err(SimulatorError::UnknownSimulator)
            }
        }
    }
}
