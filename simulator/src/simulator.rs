use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use clap::Subcommand;
use enum_dispatch::enum_dispatch;
use logger::Logger;
use option_parser::{DebugConfiguration, OptionParser};
use owo_colors::OwoColorize;
use remu_macro::{log_error, log_todo};
use remu_utils::{Disassembler, ProcessError, ProcessResult, Simulators};
use state::{reg::RegfileIo, States};

use crate::{
    difftest_ref::DifftestManager, emu::Emu, nzea::Nzea, TraceFunction, Tracer
};

#[derive(Debug, Subcommand)]
pub enum FunctionTarget {
    InstructionTrace,
    WaveTrace,
    GuiSimulator,
}

#[enum_dispatch(SimulatorEnum)]
pub trait SimulatorItem {
    fn init(&self) -> Result<(), SimulatorError> { Ok(()) }
    fn step_cycle(&mut self) -> ProcessResult<()> { log_todo!(); Ok(()) }
    fn times(&self) -> ProcessResult<()> { log_todo!(); Ok(()) }
    fn function_wave_trace(&self, _enable: bool) { log_todo!(); }
    fn function_nvboard(&self, _enable: bool) { log_todo!(); }
}

#[enum_dispatch]
pub enum SimulatorEnum {
    NEMU(Emu),
    NZEA(Nzea),
}

#[derive(Debug, snafu::Snafu)]
pub enum SimulatorError {
    #[snafu(display("Simulator Init Failed"))]
    InitFailed,
    #[snafu(display("Unknown Simulator"))]
    UnknownSimulator,
}

pub struct SimulatorCallback {
    pub instruction_complete: Box<dyn FnMut(u32, u32, u32) -> ProcessResult<()>>,
    pub difftest_skip: Box<dyn Fn()>,
    pub decode_failed: Box<dyn Fn(u32, u32)>,
    pub trap: Box<dyn Fn()>,
}

impl SimulatorCallback {
    pub fn new(
        instruction_complete: Box<dyn FnMut(u32, u32, u32) -> ProcessResult<()>>,
        difftest_skip: Box<dyn Fn()>,
        decode_failed: Box<dyn Fn(u32, u32)>,
        trap: Box<dyn Fn()>,
    ) -> Self {
        Self {
            instruction_complete,
            difftest_skip,
            decode_failed,
            trap,
        }
    }
}

impl TryFrom<(&OptionParser, States, SimulatorCallback)> for SimulatorEnum {
    type Error = SimulatorError;
    fn try_from(
        (option, states, callback): (&OptionParser, States, SimulatorCallback),
    ) -> Result<Self, Self::Error> {
        match option.cli.platform.simulator {
            Simulators::EMU(_) => Ok(SimulatorEnum::NEMU(Emu::new(option, states, callback))),
            Simulators::NZEA(_) => Ok(SimulatorEnum::NZEA(Nzea::new(option, states, callback))),
        }
    }
}

#[derive(PartialEq, Clone)]
pub enum SimulatorState {
    STOP,
    RUN,
    TRAPED(bool),
}

pub struct Simulator {
    pub state: Arc<Mutex<SimulatorState>>,
    pub dut: SimulatorEnum,

    pub states_dut: States,
    pub states_ref: States,

    pub difftest_manager: Option<Rc<RefCell<DifftestManager>>>,
    pub tracer: Rc<RefCell<Tracer>>,

    pub disassembler: Rc<RefCell<Disassembler>>,
    pub debug_config: SimulatorDebugConfig,
}

pub struct SimulatorDebugConfig {
    pub pending_instructions: Rc<RefCell<u64>>,
}

impl Simulator {
    pub fn new(
        option: &OptionParser,
        states_dut: States,

        states_ref: States,

        disasm: Rc<RefCell<Disassembler>>,
    ) -> Result<Self, SimulatorError> {
        let (itrace, wavetrace) = option.cfg.debug_config.iter().fold((false, false), |mut acc, cfg| {
            match cfg {
                DebugConfiguration::Itrace { enable } => {
                    Logger::function("ITrace", *enable);
                    acc.0 = *enable;
                }
                DebugConfiguration::WaveTrace { enable } => {
                    Logger::function("WaveTrace", *enable);
                    acc.1 = *enable;
                }
                _ => {}
            }
            acc
        });

        let pending_instructions = Rc::new(RefCell::new(0));
        let simulator_state = Arc::new(Mutex::new(SimulatorState::STOP));

        let tracer = Rc::new(RefCell::new(Tracer::new(
            itrace,
            disasm.clone(),
        )));

        let difftest_manager = option.cli.differtest.as_ref().map(|_| {
            Rc::new(RefCell::new(
                DifftestManager::new(
                    option,
                    states_dut.clone(),
                    states_ref.clone(),
                )
            ))
        });

        ctrlc::set_handler({
            let simulator_state = simulator_state.clone();
            move || {
                *simulator_state.lock().unwrap() = SimulatorState::STOP;
            }
        }).unwrap();

        let instruction_complete_callback = {
            let pending_instructions = pending_instructions.clone();
            let tracer = tracer.clone();
            let difftest_manager = difftest_manager.clone();

            Box::new(move |pc: u32, next_pc: u32, inst: u32| -> ProcessResult<()> {
                tracer.borrow().trace(pc, next_pc, inst)?;

                difftest_manager
                    .as_ref()
                    .map(|mgr| 
                        mgr.borrow_mut().step()
                    ).transpose()?;

                let mut pending = pending_instructions.borrow_mut();
                if *pending > 0 {
                    *pending -= 1;
                    if *pending == 0 {
                        return Err(ProcessError::Recoverable);
                    }
                }
                
                Ok(())
            })
        };

        let difftest_skip_callback = {
            let difftest_manager = difftest_manager.clone();
            Box::new(move || {
                if let Some(mgr) = difftest_manager.as_ref() {
                    mgr.borrow_mut().skip();
                }
            })
        };

        let decode_failed_callback = Box::new(|pc: u32, inst: u32| {
            Logger::show("Decode Failed", Logger::ERROR);
            println!("0x{:08x}: 0x{:08x}", pc.blue(), inst.purple());
        });

        let trap_callback = {
            let states_dut = states_dut.clone();
            let simulator_state = simulator_state.clone();

            Box::new(move || {
                let is_good = states_dut.regfile.read_gpr(10).unwrap() == 0;
                let msg = if is_good { Logger::SUCCESS } else { Logger::ERROR };
                Logger::show(if is_good { "Hit Good Trap" } else { "Hit Bad Trap" }, msg);
                *simulator_state.lock().unwrap() = SimulatorState::TRAPED(is_good);
            }
        )};

        let dut_callback = SimulatorCallback::new(
            instruction_complete_callback,
            difftest_skip_callback,
            decode_failed_callback,
            trap_callback,
        );
        let dut = SimulatorEnum::try_from((option, states_dut.clone(), dut_callback)).unwrap();
        dut.init()?;

        let debug_config = SimulatorDebugConfig {
            pending_instructions,
        };

        if wavetrace {
            dut.function_wave_trace(true);
        }

        Ok(Self {
            state: simulator_state,
            dut,
            states_dut,
            states_ref,
            difftest_manager,
            tracer,
            disassembler: disasm,
            debug_config,
        })
    }

    pub fn step_cycle(&mut self, count: u64) -> ProcessResult<()> {
        *self.state.lock().unwrap() = SimulatorState::RUN;
        for _ in 0..count {
            match self.state.lock().unwrap().clone() {
                SimulatorState::TRAPED(_) => {
                    log_error!("Simulator already TRAPED!");
                    return Err(ProcessError::Recoverable);
                }
                SimulatorState::STOP => return Err(ProcessError::Recoverable),
                _ => {}
            }
            self.dut.step_cycle()?;
        }
        Ok(())
    }

    pub fn step_instruction(&mut self, count: u64) -> ProcessResult<()> {
        self.debug_config.pending_instructions.replace(count);
        self.step_cycle(u64::MAX)
    }

    pub fn cmd_function_mut(&mut self, subcmd: FunctionTarget, enable: bool) -> ProcessResult<()> {
        match subcmd {
            FunctionTarget::InstructionTrace => {
                self.tracer.borrow_mut().trace_function(TraceFunction::InstructionTrace, enable);
                Logger::function("ITrace", enable);
            }
            FunctionTarget::WaveTrace => {
                self.dut.function_wave_trace(enable);
                Logger::function("WaveTrace", enable);
            }
            FunctionTarget::GuiSimulator => self.dut.function_nvboard(enable),
        }
        Ok(())
    }
}
