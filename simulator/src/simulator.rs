use std::{cell::RefCell, rc::Rc};

use logger::Logger;
use option_parser::{DebugConfiguration, OptionParser};
use owo_colors::OwoColorize;
use remu_macro::{log_error, log_todo};
use remu_utils::{Disassembler, ProcessError, ProcessResult, Simulators};
use enum_dispatch::enum_dispatch;
use state::States;

use crate::{difftest_ref::{DifftestRefApi, AnyDifftestRef}, emu::Emu};
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum FunctionTarget {
    /// The instruction trace function
    InstructionTrace,
}

#[enum_dispatch(SimulatorEnum)]
pub trait SimulatorItem {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        log_todo!();
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

pub struct SimulatorCallback {
    pub instruction_compelete: Box<dyn Fn(u32, u32)>,
    pub decode_failed: Box<dyn Fn(u32, u32)>,
    pub trap: Box<dyn Fn(bool)>,
}

impl SimulatorCallback {
    pub fn new(instruction_compelete: Box<dyn Fn(u32, u32)>, decode_failed: Box<dyn Fn(u32, u32)>, trap: Box<dyn Fn(bool)>) -> Self {
        Self {
            instruction_compelete,
            decode_failed,
            trap,
        }
    }
}

impl TryFrom<(&OptionParser, States, SimulatorCallback)> for SimulatorEnum {
    type Error = SimulatorError;

    fn try_from((option, states, callback): (&OptionParser, States, SimulatorCallback)) -> Result<Self, Self::Error> {
        let sim = option.cli.platform.simulator;
        match sim {
            Simulators::EMU => Ok(SimulatorEnum::NEMU(Emu::new(option, states, callback))),
            Simulators::NPC => {
                log_error!("NPC is not implemented yet");
                Err(SimulatorError::UnknownSimulator)
            }
        }
    }
}

#[derive(PartialEq)]
pub enum SimulatorState {
    IDLE,
    TRAPED(bool),
}

pub struct Simulator {
    pub state: Rc<RefCell<SimulatorState>>,

    pub dut: SimulatorEnum,
    pub states_dut: States,

    pub r#ref: Option<AnyDifftestRef>,
    pub states_ref: States,

    pub instruction_trace_enable: Rc<RefCell<bool>>,
    pub disaseembler: Rc<RefCell<Disassembler>>,
}

impl Simulator {
    pub fn new(option: &OptionParser, states_dut: States, states_ref: States, disasm: Rc<RefCell<Disassembler>>) -> Result<Self, SimulatorError> {
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
        let simulator_state: Rc<RefCell<SimulatorState>> = Rc::new(RefCell::new(SimulatorState::IDLE));

        let disasm_clone = disasm.clone();
        let instruction_trace_enable_clone = instruction_trace_enable.clone();
        let simulator_state_clone = simulator_state.clone();

        let instruction_compelete_callback = Box::new(move |pc: u32, inst: u32| {
            if *instruction_trace_enable_clone.borrow() == false {
                return;
            }
            let disassembler = disasm_clone.borrow();
            println!("0x{:08x}: {}", pc.blue(), disassembler.try_analize(inst, pc).purple());
        });
        
        let decode_failed_callback = Box::new(|pc: u32, inst: u32| {
            Logger::show("Decode Failed", Logger::ERROR);
            println!("0x{:08x}: 0x{:08x}", pc.blue(), inst.purple());
        });

        let trap_callback = Box::new(move |is_good: bool| {
            if is_good == false {
                Logger::show("Hit Bad Trap", Logger::ERROR);
                *simulator_state_clone.borrow_mut() = SimulatorState::TRAPED(false);
            } else {
                Logger::show("Hit Good Trap", Logger::SUCCESS);
                *simulator_state_clone.borrow_mut() = SimulatorState::TRAPED(true);
            }
        });

        let dut_callback = SimulatorCallback::new(
            instruction_compelete_callback, 

            decode_failed_callback,

            trap_callback
        );
        
        let ref_callback = SimulatorCallback::new(
            Box::new(|_: u32, _: u32| {}), 
            Box::new(|_: u32, _: u32| {}), 
            Box::new(|_: bool| {}));

        let dut = SimulatorEnum::try_from((option, states_dut.clone(), dut_callback)).unwrap();
        let r#ref = if option.cli.differtest.is_some() {
            Some(AnyDifftestRef::try_from((option, states_ref.clone(), ref_callback)).unwrap())
        } else {
            None
        };

        Ok(Self {
            state: simulator_state,

            dut,
            states_dut,

            r#ref,
            states_ref,

            instruction_trace_enable,
            disaseembler: disasm
        })
    }
    pub fn step_cycle(&mut self) -> ProcessResult<()> {
        if let SimulatorState::TRAPED(_) = *self.state.borrow() {
            log_error!("Simulator already TRAPED!");
            return Err(ProcessError::Recoverable);
        }

        self.dut.step_cycle()?;
        
        if let Some(r#ref) = &mut self.r#ref {
            r#ref.step_cycle()?;
            if r#ref.test_reg(&self.states_dut.regfile) == false {
                *self.state.borrow_mut() = SimulatorState::TRAPED(false);
                return Err(ProcessError::Recoverable);
            }
        }
        Ok(())
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
