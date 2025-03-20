use std::{cell::RefCell, rc::Rc};

use logger::Logger;
use option_parser::OptionParser;
use state::mmu::MMU;
use crate::cmd_parser::{ProcessResult, Server};

pub struct SimpleDebugger {
    server: Server,

    mmu: Rc<RefCell<MMU>>,
}

impl SimpleDebugger {
    pub fn new(cli_result: OptionParser) -> Result<Self, ()> {
        let (_isa, name) = cli_result.cli.platform.split_once('-').unwrap();

        let mmu = Rc::new(RefCell::new(MMU::new()));
        for (_isa, name, base, length, flag) in &cli_result.config {
            mmu.borrow_mut().add_memory(*base, *length, name, flag.clone()).map_err(|e| {
                Logger::show(&e.to_string(), Logger::ERROR);
            })?;
        }

        Ok(Self {
            server: Server::new(name).expect("Unable to create server"),
            mmu,
        })
    }

    pub fn mainloop(mut self) -> Result<(), ()> {
        loop {
            let line = self.server.readline();

            let line = match line {
                ProcessResult::Halt => return Ok(()),
                ProcessResult::Error => return Err(()),
                ProcessResult::Continue(line) => line,
            };

            let line = line.trim().split_whitespace().collect::<Vec<&str>>();
            if line.len() == 0 {
                continue;
            }

            Logger::show(&line[0], Logger::TRACE);
        }
    }
}
