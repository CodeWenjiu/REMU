use std::{cell::RefCell, rc::Rc};

use logger::Logger;
use option_parser::{BaseConfiguration, DebugConfiguration, MemoryConfiguration, OptionParser};
use remu_buildin::{get_buildin_img, get_reset_vector, READLINE_HISTORY_LENGTH};
use simulator::{Simulator, SimulatorImpl};
use state::States;
use crate::cmd_parser::Server;

use remu_utils::{Disassembler, ProcessError};

pub struct SimpleDebugger {
    server: Server,

    pub disassembler: Rc<RefCell<Disassembler>>,

    pub state: Rc<RefCell<States>>,

    pub simulator: Box<dyn Simulator>,
}

impl SimpleDebugger {
    pub fn new(cli_result: OptionParser) -> Result<Self, ()> {
        let isa = cli_result.cli.platform.isa;

        let mut reset_vector = get_reset_vector(isa);

        for base_config in &cli_result.cfg.base_config {
            match base_config {
                BaseConfiguration::ResetVector { value } => {
                    reset_vector = *value;
                }
            }
        }

        let disassembler = Disassembler::new(isa)?;
        let disassembler = Rc::new(RefCell::new(disassembler));

        let state = Rc::new(RefCell::new(States::new(isa, reset_vector)?));

        for mem in &cli_result.cfg.memory_config {
            match mem {
                MemoryConfiguration::MemoryRegion { name, base, size, flag } => {
                    let mmu = &mut state.borrow_mut().mmu;
                    mmu.add_memory(*base, *size, name, flag.clone()).map_err(|e| {
                        Logger::show(&e.to_string(), Logger::ERROR);
                    })?;
                }
            }
        }

        let buildin_img = get_buildin_img(isa);
        let bytes: Vec<u8> = buildin_img.iter()
            .flat_map(|&val| val.to_le_bytes().to_vec())
            .collect();

        state.borrow_mut().mmu.load(reset_vector, &bytes).map_err(|e| {
            Logger::show(&e.to_string(), Logger::ERROR);
        })?;
        let mut rl_history_length = READLINE_HISTORY_LENGTH;

        for debug_config in &cli_result.cfg.debug_config {
            match debug_config {
                DebugConfiguration::Readline { history } => {
                    rl_history_length = *history;
                }

                _ => {
                }
            }
        }

        let simulator = Box::new(SimulatorImpl::try_from((&cli_result, state.clone(), disassembler.clone())).map_err(|e| {
            Logger::show(&e.to_string(), Logger::ERROR);
        })?);

        Ok(Self {
            server: Server::new(cli_result.cli.platform.simulator, rl_history_length).expect("Unable to create server"),
            disassembler,
            state,
            simulator,
        })
    }

    pub fn mainloop(mut self) -> Result<(), ()> {
        loop {
            macro_rules! handle_result {
                ($result:expr) => {
                    match $result {
                        Err(ProcessError::Recoverable) => continue,
                        Err(ProcessError::GracefulExit) => return Ok(()),
                        Err(ProcessError::Fatal) => return Err(()),
                        Ok(value) => value,
                    }
                };
            }

            let cmd = handle_result!(self.server.get_parse());
            handle_result!(self.execute(cmd.command));
        }
    }
}
