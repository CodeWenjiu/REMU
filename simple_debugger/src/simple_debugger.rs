use std::{cell::RefCell, rc::Rc};

use logger::Logger;
use option_parser::OptionParser;
use state::mmu::MMU;
use crate::cmd_parser::{Cmds, InfoCmds, MemoryCmds, ProcessResult, Server};

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
            let cmd = self.server.get_parse();

            let cmd = match cmd {
                ProcessResult::Halt => return Ok(()),
                ProcessResult::Error => return Err(()),
                ProcessResult::Continue(cmd) => cmd,
            };

            match cmd.command {
                Cmds::Info { subcmd } => {
                    match subcmd {
                        InfoCmds::Memory { subcmd } => {
                            match subcmd {
                                MemoryCmds::ShowMemoryMap {} => {
                                    self.mmu.borrow().show_memory_map();
                                }

                                _ => {
                                    Logger::todo();
                                }
                            }
                        }

                        _ => {
                            Logger::todo();
                        }
                    }
                }

                _ => {
                    Logger::todo();
                }
            }
        }
    }
}
