use std::{cell::RefCell, rc::Rc};

use logger::Logger;
use option_parser::{DebugConfiguration, OptionParser};
use state::mmu::MMU;
use crate::cmd_parser::{ProcessResult, Server};

pub struct SimpleDebugger {
    server: Server,

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
            mmu,
        })
    }

    pub fn mainloop(mut self) -> Result<(), ()> {
        loop {
            let cmd = self.server.get_parse();

            let cmd = match cmd {
                ProcessResult::Halt => return Ok(()),
                ProcessResult::Error => return Err(()),
                ProcessResult::Continue(cmd) => cmd,
            };

            self.execute(cmd.command)?;
        }
    }
}
