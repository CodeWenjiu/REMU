use std::{cell::RefCell, rc::Rc, sync::{Arc, Mutex}};

use clap::Subcommand;
use enum_dispatch::enum_dispatch;
use logger::Logger;
use option_parser::{DebugConfiguration, OptionParser};
use owo_colors::OwoColorize;
use remu_macro::{log_err, log_error, log_todo};
use remu_utils::{Disassembler, ProcessError, ProcessResult, Simulators};
use state::{reg::RegfileIo, CheckFlags4reg, States};

use crate::{
    difftest_ref::{AnyDifftestRef, DifftestRefBuildInApi, DifftestRefFfiApi},
    emu::Emu, nzea::Nzea,
};

/// Available function targets for enabling/disabling simulator features
#[derive(Debug, Subcommand)]
pub enum FunctionTarget {
    /// The instruction trace function - displays executed instructions
    InstructionTrace,
    /// The Wave trace function - displays the waveforms of signals(basically only suit for HDL simulator)
    WaveTrace,
    /// GUI Simulator, for now it direct to `NVBOARD`
    GuiSimulator,
}

/// Common interface for all simulator implementations
#[enum_dispatch(SimulatorEnum)]
pub trait SimulatorItem {
    /// Some init code
    fn init(&self) -> Result<(), SimulatorError> {
        Ok(())
    }

    /// Execute a single cycle in the simulator
    fn step_cycle(&mut self) -> ProcessResult<()> {
        log_todo!();
        Ok(())
    }

    /// Show Times
    fn times(&self) -> ProcessResult<()> {
        log_todo!();
        Ok(())
    }

    fn function_wave_trace(&self, _enable: bool) {
        log_todo!();
    }

    fn function_nvboard(&self, _enable: bool) {
        log_todo!();
    }
}

/// Enum of available simulator implementations
#[enum_dispatch]
pub enum SimulatorEnum {
    NEMU(Emu),
    NZEA(Nzea),
}

/// Errors that can occur during simulator operations
#[derive(Debug, snafu::Snafu)]
pub enum SimulatorError {
    /// Simulator init failed
    #[snafu(display("Simulator Init Failed"))]
    InitFailed,

    /// Requested simulator is not implemented or not available
    #[snafu(display("Unknown Simulator"))]
    UnknownSimulator,
}

/// Callbacks for simulator events
pub struct SimulatorCallback {
    /// Called when an instruction is successfully executed
    pub instruction_compelete: Box<dyn FnMut(u32, u32) -> ProcessResult<()>>,
    /// Called when an need to skip difftest
    pub difftest_skip: Box<dyn Fn()>,
    /// Called when instruction decoding fails
    pub decode_failed: Box<dyn Fn(u32, u32)>,
    /// Called when a trap is encountered (true = good trap, false = bad trap)
    pub trap: Box<dyn Fn(bool)>,
}

impl SimulatorCallback {
    /// Create a new SimulatorCallback with the specified handlers
    pub fn new(
        instruction_compelete: Box<dyn FnMut(u32, u32) -> ProcessResult<()>>,
        difftest_skip: Box<dyn Fn()>,
        decode_failed: Box<dyn Fn(u32, u32)>,
        trap: Box<dyn Fn(bool)>,
    ) -> Self {
        Self {
            instruction_compelete,
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
        let sim = option.cli.platform.simulator;
        match sim {
            Simulators::EMU => Ok(SimulatorEnum::NEMU(Emu::new(option, states, callback))),
            Simulators::NZEA => Ok(SimulatorEnum::NZEA(Nzea::new(option, states, callback)))
        }
    }
}

/// Represents the current state of the simulator
#[derive(PartialEq, Clone)]
pub enum SimulatorState {
    /// Simulator is ready to execute
    STOP,
    /// Simulator is running
    RUN,
    /// Simulator has encountered a trap (true = good trap, false = bad trap)
    TRAPED(bool),
}

/// Main simulator that coordinates execution and testing
pub struct Simulator {
    /// Current state of the simulator
    pub state: Arc<Mutex<SimulatorState>>,

    /// Device Under Test - the simulator being tested
    pub dut: SimulatorEnum,
    /// State of the Device Under Test
    pub states_dut: States,

    /// Reference simulator for comparison (if differtest is enabled)
    pub r#ref: Option<Rc<RefCell<AnyDifftestRef>>>,
    /// State of the reference simulator
    pub states_ref: States,

    /// Disassembler for instruction tracing
    pub disaseembler: Rc<RefCell<Disassembler>>,

    /// Debug configuration for the simulator
    pub debug_config: SimulatorDebugConfig,
}

pub struct SimulatorDebugConfig {
    /// Flag to enable/disable instruction tracing
    pub instruction_trace_enable: Rc<RefCell<bool>>,
    /// Counter for remaining instructions to execute
    pub pending_instructions: Rc<RefCell<u64>>,
    /// Memory addresses to watch for changes
    pub memory_watch_points: Rc<RefCell<Vec<u32>>>,
}

impl Simulator {
    /// Create a new Simulator instance
    pub fn new(
        option: &OptionParser,
        states_dut: States,
        states_ref: States,
        disasm: Rc<RefCell<Disassembler>>,
    ) -> Result<Self, SimulatorError> {
        let pending_instructions = Rc::new(RefCell::new(0));
        let memory_watch_points = Rc::new(RefCell::new(vec![]));

        // Create a minimal callback for the reference simulator
        let ref_callback = SimulatorCallback::new(
            Box::new(|_: u32, _: u32| Ok(())),
            Box::new(|| {}),
            Box::new(|_: u32, _: u32| {}),
            Box::new(|_: bool| {}),
        );

        // Initialize reference simulator if differtest is enabled
        let r#ref = if option.cli.differtest.is_some() {
            Some(Rc::new(RefCell::new(
                AnyDifftestRef::try_from((option, states_ref.clone(), ref_callback)).unwrap(),
            )))
        } else {
            None
        };

        // Parse debug configuration options
        let mut itrace = false;
        let mut wavetrace = false;
        for debug_config in &option.cfg.debug_config {
            match debug_config {
                DebugConfiguration::Itrace { enable } => {
                    Logger::function("ITrace", *enable);
                    itrace = *enable;
                }

                DebugConfiguration::WaveTrace { enable } => {
                    Logger::function("WaveTrace", *enable);
                    wavetrace = *enable;
                }

                DebugConfiguration::Readline { history: _ } => {
                    // Ignore readline history configuration here
                }
            }
        }

        let instruction_trace_enable = Rc::new(RefCell::new(itrace));
        let simulator_state: Arc<Mutex<SimulatorState>> =
            Arc::new(Mutex::new(SimulatorState::STOP));

        // Set Signal Handle to change simulator_state
        let simulator_state_clone0 = simulator_state.clone();
        ctrlc::set_handler(move || {
            // Set the simulator state to TRAPED(true) when Ctrl+C is pressed
            *simulator_state_clone0.lock().unwrap() = SimulatorState::STOP;
        }).unwrap();

        // Create clones for use in callbacks
        let disasm_clone = disasm.clone();
        let instruction_trace_enable_clone = instruction_trace_enable.clone();
        let simulator_state_clone1 = simulator_state.clone();
        let simulator_state_clone2 = simulator_state.clone();
        let r#ref_clone = r#ref.clone();
        let mut state_ref_clone = states_ref.clone();
        let mut state_dut_clone = states_dut.clone();
        let pending_instructions_clone = pending_instructions.clone();
        let memory_watch_points_clone = memory_watch_points.clone();
        let is_difftest_skip = Rc::new(RefCell::new(false));
        let is_difftest_skip_clone = is_difftest_skip.clone();

        // Callback for instruction completion
        let instruction_compelete_callback = Box::new(move |pc: u32, inst: u32| -> ProcessResult<()> {
            // Print instruction trace if enabled
            if *instruction_trace_enable_clone.borrow() {
                let disassembler = disasm_clone.borrow();
                println!(
                    "0x{:08x}: {}",
                    pc.blue(),
                    disassembler.try_analize(inst, pc).purple()
                );
            }
            
            // create an clouser to difftest 
            let mut difftest_step = || -> ProcessResult<()> {
                // Execute the instruction in the reference simulator
                if r#ref_clone.is_none() {
                    return Ok(());
                }
                let mut ref_mut = r#ref_clone.as_ref().unwrap().borrow_mut();
                match &mut *ref_mut {
                    AnyDifftestRef::FFI(r_ref) => {
                        r_ref.step_cycle()?;
                        r_ref.test_reg(&state_dut_clone.regfile).map_err(|e| {
                            *simulator_state_clone1.lock().unwrap() = SimulatorState::TRAPED(false);
                            e
                        })?;

                        // Check memory watchpoints
                        let mut mem_diff_msg = vec![];
                        for addr in memory_watch_points_clone.borrow().iter() {
                            let dut_data = log_err!(
                                state_dut_clone.mmu.read(*addr, state::mmu::Mask::Word),
                                ProcessError::Recoverable
                            )?.1;
                            mem_diff_msg.push((*addr, dut_data));
                        }
                        r_ref.test_mem(mem_diff_msg)?;
                    }
        
                    AnyDifftestRef::BuildIn(r_ref) => {
                        r_ref.instruction_compelete()?;

                        state_ref_clone.regfile.check(&state_dut_clone.regfile, CheckFlags4reg::gpr.union(CheckFlags4reg::pc))?;
                    }
                }
                
                Ok(())
            };

            if *is_difftest_skip_clone.borrow() == true {
                if let Some(r#ref) = &r#ref_clone {
                    *is_difftest_skip_clone.borrow_mut() = false;
                    match &mut *r#ref.borrow_mut() {
                        AnyDifftestRef::FFI(r_ref) => {
                            r_ref.set_ref(&state_dut_clone.regfile);
                        }
                        AnyDifftestRef::BuildIn(_) => {
                            state_ref_clone.regfile.set_reg(&state_dut_clone.regfile);
                        }
                    }
                }
            } else {
                difftest_step()?;
            }

            // Handle instruction counting for step_instruction
            let mut pending = pending_instructions_clone.borrow_mut();
            if *pending > 0 {
                *pending -= 1;
                if *pending == 0 {
                    return Err(ProcessError::Recoverable);
                }
            }

            Ok(())
        });

        let is_difftest_skip_clone = is_difftest_skip.clone();
        // Callback for difftest skip
        let difftest_skip_callback = Box::new(move || {
            *is_difftest_skip_clone.borrow_mut() = true;
        });

        // Callback for decode failures
        let decode_failed_callback = Box::new(|pc: u32, inst: u32| {
            Logger::show("Decode Failed", Logger::ERROR);
            println!("0x{:08x}: 0x{:08x}", pc.blue(), inst.purple());
        });

        // Callback for trap events
        let trap_callback = Box::new(move |is_good: bool| {
            if !is_good {
                Logger::show("Hit Bad Trap", Logger::ERROR);
                *simulator_state_clone2.lock().unwrap() = SimulatorState::TRAPED(false);
            } else {
                Logger::show("Hit Good Trap", Logger::SUCCESS);
                *simulator_state_clone2.lock().unwrap() = SimulatorState::TRAPED(true);
            }
        });

        // Create the DUT callback and simulator
        let dut_callback = SimulatorCallback::new(
            instruction_compelete_callback,
            difftest_skip_callback,
            decode_failed_callback,
            trap_callback,
        );
        let dut = SimulatorEnum::try_from((option, states_dut.clone(), dut_callback)).unwrap();
        dut.init()?;

        let debug_config = SimulatorDebugConfig {
            instruction_trace_enable,
            pending_instructions,
            memory_watch_points,
        };

        if wavetrace {
            dut.function_wave_trace(true);
        }

        Ok(Self {
            state: simulator_state,
            dut,
            states_dut,
            r#ref,
            states_ref,
            disaseembler: disasm,
            debug_config,
        })
    }

    /// Execute a specified number of cycles
    pub fn step_cycle(&mut self, count: u64) -> ProcessResult<()> {
        *self.state.lock().unwrap() = SimulatorState::RUN;

        // Execute the specified number of cycles
        for _ in 0..count {
            let state = self.state.lock().unwrap().clone();
            if let SimulatorState::TRAPED(_) = state {
                log_error!("Simulator already TRAPED!");
                return Err(ProcessError::Recoverable);
            } else if SimulatorState::STOP == state {
                return Err(ProcessError::Recoverable);
            }

            self.dut.step_cycle()?;
        }

        Ok(())
    }

    /// Execute a specified number of instructions
    pub fn step_instruction(&mut self, count: u64) -> ProcessResult<()> {
        // Set the number of instructions to execute
        self.debug_config.pending_instructions.replace(count);

        // Run until all instructions are executed or an error occurs
        // Using u64::MAX as the cycle count ensures we run until the instruction count is reached
        self.step_cycle(u64::MAX)?;

        Ok(())
    }

    /// Enable or disable a simulator function
    pub fn cmd_function_mut(&mut self, subcmd: FunctionTarget, enable: bool) -> ProcessResult<()> {
        match subcmd {
            FunctionTarget::InstructionTrace => {
                self.debug_config.instruction_trace_enable.replace(enable);
                Logger::function("ITrace", enable);
            }
            FunctionTarget::WaveTrace => {
                self.dut.function_wave_trace(enable);
                Logger::function("WaveTrace", enable);
            }
            FunctionTarget::GuiSimulator => {
                self.dut.function_nvboard(enable);
            }
        }

        Ok(())
    }
}
