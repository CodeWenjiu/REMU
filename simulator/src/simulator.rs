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

#[enum_dispatch(SimulatorEnum)]
pub trait SimulatorItem {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        Logger::todo();
        Ok(())
    }

    fn add_inst_compelete_callback(&mut self, _target: FunctionTarget) -> ProcessResult<()> {
        Logger::todo();
        Ok(())
    }
}

#[enum_dispatch]
pub enum SimulatorEnum {
    NEMU(Emu),
}

#[derive(Debug, snafu::Snafu)]
pub enum SimulatorError {
    #[snafu(display("Unknown Simulator"))]
    UnknownSimulator,
}

impl TryFrom<(&OptionParser, States)> for SimulatorEnum {
    type Error = SimulatorError;

    fn try_from((option, states): (&OptionParser, States)) -> Result<Self, Self::Error> {
        let sim = option.cli.platform.simulator;
        match sim {
            Simulators::EMU => Ok(SimulatorEnum::NEMU(Emu::new(option, states))),
            Simulators::NPC => {
                Logger::show("NPC is not implemented yet", Logger::ERROR);
                Err(SimulatorError::UnknownSimulator)
            }
        }
    }
}

pub struct Simulator {
    pub dut: SimulatorEnum,

    pub instruction_trace_enable: bool,
    pub disaseembler: Rc<RefCell<Disassembler>>,
}

impl Simulator {
    pub fn new(option: &OptionParser, states: States, disasm: Rc<RefCell<Disassembler>>) -> Result<Self, SimulatorError> {
        let dut = SimulatorEnum::try_from((option, states))?;
        Ok(Self { 
            dut,
            instruction_trace_enable: false,
            disaseembler: disasm
        })
    }

    pub fn step_cycle(&mut self) -> ProcessResult<()> {
        self.dut.step_cycle()
    }

    pub fn cmd_function_mut(&mut self, subcmd: FunctionTarget, enable: bool) -> ProcessResult<()> {
        match subcmd {
            FunctionTarget::InstructionTrace => {
                self.instruction_trace_enable = enable;
            }
        }
        
        Ok(())
    }
}
