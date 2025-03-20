use std::{cell::RefCell, rc::Rc};

use logger::Logger;
use option_parser::{DebugConfiguration, OptionParser};
use state::mmu::MMU;
use crate::{cmd_parser::Server, debug::Disassembler, ProcessError};

pub struct SimpleDebugger {
    server: Server,

    pub disassembler: Disassembler,

    pub mmu: Rc<RefCell<MMU>>,
}

impl SimpleDebugger {
    pub fn new(cli_result: OptionParser) -> Result<Self, ()> {
        let (_isa, name) = cli_result.cli.platform.split_once('-').unwrap();

        let mmu = Rc::new(RefCell::new(MMU::new()));
        for (_isa, name, base, length, flag) in &cli_result.memory_config {
            mmu.borrow_mut().add_memory(*base, *length, name, flag.clone()).map_err(|e| {
                Logger::show(&e.to_string(), Logger::ERROR);
            })?;
        }

        let mut rl_history_length = 100;

        for debug_config in &cli_result.debug_config {
            match debug_config {
                DebugConfiguration::Readline { history } => {
                    rl_history_length = *history;
                }
            }
        }

        Ok(Self {
            server: Server::new(name, rl_history_length).expect("Unable to create server"),
            disassembler: Disassembler::new("riscv64-unknown-linux-gnu")?,
            mmu,
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
