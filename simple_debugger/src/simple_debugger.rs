use std::{cell::RefCell, rc::Rc};

use logger::Logger;
use option_parser::{DebugConfiguration, OptionParser};
use simulator::{emu::Emu, Simulator};
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
    fn isa2triple(isa: &str) -> Result<&str, ()> {
        match isa {
            "rv32e" => Ok("riscv64-unknown-linux-gnu"),
            "rv32i" => Ok("riscv64-unknown-linux-gnu"),
            "rv32im" => Ok("riscv64-unknown-linux-gnu"),

            _ => {
                Logger::show(&format!("Unknown ISA: {}", isa), Logger::ERROR);
                Err(())
            }
        }
    }

    pub fn new(cli_result: OptionParser) -> Result<Self, ()> {
        let (isa, name) = cli_result.cli.platform.split_once('-').unwrap();

        let disassembler = Disassembler::new(Self::isa2triple(isa)?)?;
        let disassembler = Rc::new(RefCell::new(disassembler));

        let regfile_io = regfile_io_factory(isa).map_err(|_| {
            Logger::show(&format!("Unknown ISA: {}", isa), Logger::ERROR);
        })?;
        let regfile = Rc::new(RefCell::new(regfile_io));

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
            disassembler,
            regfile,
            mmu,
            simulator: Box::new(Emu::new()),
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
