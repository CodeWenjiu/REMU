use std::{cell::RefCell, rc::Rc};

use logger::Logger;
use option_parser::{DebugConfiguration, OptionParser};
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

impl TryFrom<(&OptionParser, States, Box<dyn Fn(u32, u32)>)> for SimulatorEnum {
    type Error = SimulatorError;

    fn try_from((option, states, callback): (&OptionParser, States, Box<dyn Fn(u32, u32)>)) -> Result<Self, Self::Error> {
        let sim = option.cli.platform.simulator;
        match sim {
            Simulators::EMU => Ok(SimulatorEnum::NEMU(Emu::new(option, states, callback))),
            Simulators::NPC => {
                Logger::show("NPC is not implemented yet", Logger::ERROR);
                Err(SimulatorError::UnknownSimulator)
            }
        }
    }
}

pub struct Simulator {
    pub dut: SimulatorEnum,

    pub instruction_trace_enable: Rc<RefCell<bool>>,
    pub disaseembler: Rc<RefCell<Disassembler>>,
}

impl Simulator {
    pub fn new(option: &OptionParser, states_dut: States, _states_ref: States, disasm: Rc<RefCell<Disassembler>>) -> Result<Self, SimulatorError> {
        let mut itrace = false;
        for debug_config in &option.cfg.debug_config {
            match debug_config {
                DebugConfiguration::Itrace { enable } => {
                    Logger::function("ITrace", *enable);
                    itrace = *enable;
                }

                DebugConfiguration::Readline { history } => {
                    let _ = history;
                }
            }
        }

        let instruction_trace_enable = Rc::new(RefCell::new(itrace));

        let disasm_clone = disasm.clone();
        let instruction_trace_enable_clone = instruction_trace_enable.clone();

        let callback: Box<dyn Fn(u32, u32)> = Box::new(move |pc: u32, inst: u32| {
            if *instruction_trace_enable_clone.borrow() == false {
                Logger::debug();
                return;
            }
            let disassembler = disasm_clone.borrow();
            Logger::show(&format!("{}", disassembler.try_analize(inst, pc)).to_string(), Logger::INFO);
        });
        
        let dut = SimulatorEnum::try_from((option, states_dut, callback)).unwrap();

        Ok(Self {
            dut,
            instruction_trace_enable,
            disaseembler: disasm
        })
    }

    pub fn step_cycle(&mut self) -> ProcessResult<()> {
        self.dut.step_cycle()
    }

    pub fn cmd_function_mut(&mut self, subcmd: FunctionTarget, enable: bool) -> ProcessResult<()> {
        match subcmd {
            FunctionTarget::InstructionTrace => {
                self.instruction_trace_enable.replace(enable);
                Logger::show(&format!("{}", enable).to_string(), Logger::DEBUG);
            }
        }

        Ok(())
    }
}
