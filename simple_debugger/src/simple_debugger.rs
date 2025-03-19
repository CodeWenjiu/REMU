use logger::Logger;
use option_parser::OptionParser;
use crate::cmd_parser::{ProcessResult, Server};

pub struct SimpleDebugger {
    server: Server
}

impl SimpleDebugger {
    pub fn new(cli_result: OptionParser) -> Self {
        let (_isa, name) = cli_result.cli.platform.split_once('-').unwrap();

        Self {
            server: Server::new(name).expect("Unable to create server")
        }
    }

    pub fn mainloop(mut self) -> Result<(), ()> {
        loop {
            let line = self.server.readline();

            let line = match line {
                ProcessResult::Halt => return Ok(()),
                ProcessResult::Error => return Err(()),
                ProcessResult::Continue(line) => line,
            };

            Logger::show(&line, Logger::TRACE);
        }
    }
}
