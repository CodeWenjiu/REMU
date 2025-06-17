use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use cfg_if::cfg_if;
use clap::Subcommand;
use enum_dispatch::enum_dispatch;
use logger::Logger;
use option_parser::OptionParser;
use remu_buildin::get_buildin_img;
use remu_macro::{log_err, log_error, log_todo};
use remu_utils::{DifftestRef, EmuSimulators, ItraceConfigtionalWrapper, ProcessError, ProcessResult, Simulators};
use state::{reg::RegfileIo, States};

use crate::{
    difftest_ref::DifftestManager, emu::EmuWrapper, nzea::Nzea, Tracer
};

#[derive(Debug, Subcommand)]
pub enum FunctionTarget {
    #[cfg(feature = "ITRACE")]
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
    Emu(EmuWrapper),
    NZEA(Nzea),
}

#[derive(Debug, snafu::Snafu)]
pub enum SimulatorError {
    #[snafu(display("Simulator Init Failed"))]
    InitFailed,
    #[snafu(display("Unknown Simulator"))]
    UnknownSimulator,
}

impl SimulatorEnum {
    pub fn new(option: &OptionParser, states: States, callback: SimulatorCallback) -> Self {
        match option.cli.platform.simulator {
            Simulators::EMU(target) => {
                let emu = match target {
                    EmuSimulators::DM => EmuWrapper::new_dm(option, states, callback),
                    EmuSimulators::SC => EmuWrapper::new_sc(option, states, callback),
                    EmuSimulators::PL => EmuWrapper::new_pl(option, states, callback),
                };
                SimulatorEnum::Emu(emu)
            },
            Simulators::NZEA(_) => SimulatorEnum::NZEA(Nzea::new(option, states, callback)),
        }
    }
}

pub struct SimulatorCallback {
    pub instruction_complete: Box<dyn FnMut(u32, u32, u32) -> ProcessResult<()>>,
    pub difftest_skip: Box<dyn Fn(u32)>,
    pub trap: Box<dyn Fn()>,

    pub instruction_fetch: Box<dyn Fn()>,
    pub load_store: Box<dyn Fn()>,
}

impl SimulatorCallback {
    pub fn new(
        instruction_complete: Box<dyn FnMut(u32, u32, u32) -> ProcessResult<()>>,
        difftest_skip: Box<dyn Fn(u32)>,
        trap: Box<dyn Fn()>,
        instruction_fetch: Box<dyn Fn()>,
        load_store: Box<dyn Fn()>,
    ) -> Self {
        Self {
            instruction_complete,
            difftest_skip,
            trap,
            instruction_fetch,
            load_store,
        }
    }
}

#[derive(PartialEq, Clone)]
pub enum SimulatorState {
    STOP,
    RUN,
    TRAPED(bool),
}

cfg_if! {
    if #[cfg(feature = "ITRACE")] {
        use crate::TraceFunction;
    } else {
        use logger::FeatureState;
    }
}

pub struct Simulator {
    pub state: Arc<Mutex<SimulatorState>>,
    pub dut: SimulatorEnum,

    pub states_dut: States,
    pub states_ref: States,

    pub difftest_manager: Option<Rc<RefCell<DifftestManager>>>,
    pub tracer: Rc<RefCell<Tracer>>,

    pub conditional: ItraceConfigtionalWrapper,
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

        conditional: ItraceConfigtionalWrapper,
    ) -> Result<Self, SimulatorError> {
        let debug_config = &option.cfg.debug_config;
        
        let wavetrace = debug_config.wave_trace_enable;

        cfg_if!{
            if #[cfg(feature = "ITRACE")] {
                let itrace = debug_config.itrace_enable;
                Logger::function("ITrace", itrace.into());
            } else {
                Logger::function("ITrace", FeatureState::Disabled);
            }
        }

        Logger::function("WaveTrace", wavetrace.into());

        let pending_instructions = Rc::new(RefCell::new(0));
        let simulator_state = Arc::new(Mutex::new(SimulatorState::STOP));

        let tracer = Rc::new(RefCell::new(Tracer::new(
            #[cfg(feature = "ITRACE")]
            itrace,
            conditional.clone(),
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

            // ignore: simulator could be everything, let arguments more stable will be better
            Box::new(move |_pc: u32, next_pc: u32, _inst: u32| -> ProcessResult<()> {
                #[cfg(feature = "ITRACE")]
                tracer.borrow().trace(_pc, _inst)?;

                difftest_manager
                    .as_ref()
                    .map(|mgr| {
                        mgr.borrow_mut().instruction_complete();
                    });

                tracer.borrow().check_breakpoint(next_pc)?;

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
            Box::new(move |val: u32| {
                if let Some(mgr) = difftest_manager.as_ref() {
                    mgr.borrow_mut().step_skip(val);
                }
            })
        };

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

        let instruction_fetch_callback = {
            let difftest_manager = difftest_manager.clone();
            Box::new(move || {
                difftest_manager.as_ref().map(|mgr| mgr.borrow_mut().instruction_fetch());
            })
        };

        let load_store_callback = {
            let difftest_manager = difftest_manager.clone();
            Box::new(move || {
                difftest_manager.as_ref().map(|mgr| mgr.borrow_mut().load_store());
            })
        };

        let dut_callback = SimulatorCallback::new(
            instruction_complete_callback,
            difftest_skip_callback,
            trap_callback,
            instruction_fetch_callback,
            load_store_callback,
        );
        let dut = SimulatorEnum::new(option, states_dut.clone(), dut_callback);
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
            conditional,
            debug_config,
        })
    }

    pub fn load_memory(&mut self, cli_result: &OptionParser) -> ProcessResult<()> {
        let isa = cli_result.cli.platform.isa;

        let reset_vector = cli_result.cfg.platform_config.reset_vector;

        if cli_result.cli.additional_bin.is_some() {
            let bin = cli_result.cli.additional_bin.as_ref().unwrap();

            let bin_path = &bin.file_path;
            let bytes = log_err!(std::fs::read(bin_path)).unwrap();
            log_err!(self.states_dut.mmu.load(bin.load_addr, &bytes)).unwrap();

            match cli_result.cli.differtest {
                Some(DifftestRef::SingleCycle(_)) => {
                    log_err!(self.states_ref.mmu.load(0x80100000, &bytes)).unwrap();
                }

                _ => ()
            }
        };

        let buildin_img = get_buildin_img(isa);

        let bytes = if cli_result.cli.primary_bin.is_some() {
            let bin = cli_result.cli.primary_bin.as_ref().unwrap();
            let bytes = log_err!(std::fs::read(bin))
                .map_err(|e| {
                    Logger::show(
                        &format!("Unable to read binary image {}", bin).to_string(),
                        Logger::ERROR,
                    );
                    e
                })
                .unwrap();

            Logger::show(
                &format!("Loading binary image {} size: {}", bin, bytes.len() / 4).to_string(),
                Logger::INFO,
            );

            bytes
        } else {
            let bytes: Vec<u8> = buildin_img
                .iter()
                .flat_map(|&val| val.to_le_bytes().to_vec())
                .collect();

            Logger::show(
                "No binary image specified, using buildin image.",
                Logger::WARN,
            );

            bytes
        };

        log_err!(self.states_dut.mmu.load(reset_vector, &bytes)).unwrap();

        match cli_result.cli.differtest {
            Some(DifftestRef::SingleCycle(_)) => {
                log_err!(self.states_ref.mmu.load(reset_vector, &bytes)).unwrap();
            }
            Some(DifftestRef::Pipeline(_)) => {
                log_err!(self.states_ref.mmu.load(reset_vector, &bytes)).unwrap();
            }
            Some(DifftestRef::FFI(_)) => {
                self.difftest_manager.as_ref().unwrap().borrow_mut().init(&self.states_dut.regfile, bytes, reset_vector);
            }
            None => ()
        }

        Ok(())
    }

    fn check_state(&self) -> ProcessResult<()> {
        match self.state.lock().unwrap().clone() {
            SimulatorState::TRAPED(_) => {
                log_error!("Simulator already TRAPED!");
                return Err(ProcessError::Recoverable);
            }
            SimulatorState::STOP => {
                return Err(ProcessError::Recoverable);
            }
            _ => {}
        }
        Ok(())
    }

    pub fn step_cycle(&mut self, count: u64) -> ProcessResult<()> {
        *self.state.lock().unwrap() = SimulatorState::RUN;
        let mut state_check_count = 0;
        for _ in 0..count {
            if state_check_count % 10000 == 0 {
                self.check_state()?;
                state_check_count = 0;
            }
            state_check_count += 1;

            self.dut.step_cycle().and(
            
        self.difftest_manager
            .as_ref()
            .map(|mgr| 
                mgr.borrow_mut().step_cycle()
            ).transpose()
            
            )?;
        }
        Ok(())
    }

    pub fn step_instruction(&mut self, count: u64) -> ProcessResult<()> {
        self.debug_config.pending_instructions.replace(count);
        self.step_cycle(u64::MAX)
    }

    pub fn cmd_function_mut(&mut self, subcmd: FunctionTarget, enable: bool) -> ProcessResult<()> {
        match subcmd {
            #[cfg(feature = "ITRACE")]
            FunctionTarget::InstructionTrace => {
                self.tracer.borrow_mut().trace_function(TraceFunction::InstructionTrace, enable);
                Logger::function("ITrace", enable.into());
            }
            FunctionTarget::WaveTrace => {
                self.dut.function_wave_trace(enable);
                Logger::function("WaveTrace", enable.into());
            }
            FunctionTarget::GuiSimulator => self.dut.function_nvboard(enable),
        }
        Ok(())
    }
}
