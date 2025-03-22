use std::{cell::RefCell, rc::Rc};

use logger::Logger;
use option_parser::{BaseConfiguration, DebugConfiguration, MemoryConfiguration, OptionParser};
use remu_buildin::{get_buildin_img, get_reset_vector, READLINE_HISTORY_LENGTH};
use simulator::{Simulator, SimulatorImpl};
use state::{mmu::MMU, reg::{regfile_io_factory, RegfileIo}};
use crate::{cmd_parser::Server, debug::Disassembler};

use remu_utils::ProcessError;

pub struct SimpleDebugger {
    server: Server,

    pub disassembler: Rc<RefCell<Disassembler>>,

    pub regfile: Rc<RefCell<Box<dyn RegfileIo>>>,
    pub mmu: Rc<RefCell<MMU>>,

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

        let regfile_io = regfile_io_factory(isa, reset_vector)?;
        let regfile = Rc::new(RefCell::new(regfile_io));

        let mmu = Rc::new(RefCell::new(MMU::new()));
        for mem in &cli_result.cfg.memory_config {
            match mem {
                MemoryConfiguration::MemoryRegion { name, base, size, flag } => {
                    mmu.borrow_mut().add_memory(*base, *size, name, flag.clone()).map_err(|e| {
                        Logger::show(&e.to_string(), Logger::ERROR);
                    })?;
                }
            }
        }

        mmu.borrow_mut().load(reset_vector, get_buildin_img(isa).as_slice()).map_err(|e| {
            Logger::show(&e.to_string(), Logger::ERROR);
        })?;
        
        let mut rl_history_length = READLINE_HISTORY_LENGTH;

        for debug_config in &cli_result.cfg.debug_config {
            match debug_config {
                DebugConfiguration::Readline { history } => {
                    rl_history_length = *history;
                }
            }
        }

        let simulator = Box::new(SimulatorImpl::from(&cli_result.cli.platform));

        Ok(Self {
            server: Server::new(cli_result.cli.platform.simulator, rl_history_length).expect("Unable to create server"),
            disassembler,
            regfile,
            mmu,
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
