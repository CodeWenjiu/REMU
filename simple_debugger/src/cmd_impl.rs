use logger::Logger;
use owo_colors::OwoColorize;
use state::mmu::Mask;

use crate::{cmd_parser::{Cmds, InfoCmds, MemoryCmds, RegisterCmds}, ProcessError, ProcessResult, SimpleDebugger};

impl SimpleDebugger {
    fn cmd_info (&mut self, subcmd: InfoCmds) -> ProcessResult<()> {
        match subcmd {
            InfoCmds::Memory { subcmd } => {
                self.cmd_memory(subcmd)?;
            }

            InfoCmds::Register { subcmd } => {
                self.cmd_register(subcmd)?;
            }
        }

        Ok(())
    }

    fn cmd_register (&mut self, subcmd: Option<RegisterCmds>) -> ProcessResult<()> {
        match subcmd {
            Some(RegisterCmds::CSR { index }) => {
                self.regfile.borrow().print_csr(index);
            }

            Some(RegisterCmds::GPR { index }) => {
                self.regfile.borrow().print_gpr(index);
            }

            Some(RegisterCmds::PC {}) => {
                self.regfile.borrow().print_pc();
            }

            None => {
                self.regfile.borrow().print_pc();
                self.regfile.borrow().print_gpr(None);
                self.regfile.borrow().print_csr(None);
            }
        }

        Ok(())
    }

    fn cmd_memory (&mut self, subcmd: MemoryCmds) -> ProcessResult<()> {
        match subcmd {
            MemoryCmds::ShowMemoryMap {} => {
                self.mmu.borrow().show_memory_map();
            }

            MemoryCmds::Examine { addr, length } => {
                for i in 0..length {
                    let i = i as u32;
                    let data = self.mmu.borrow_mut().read(addr + i, Mask::None)
                        .map_err(|e| {
                            Logger::show(&e.to_string(), Logger::ERROR);
                            ProcessError::Recoverable
                        })?;

                    let try_parse_string: String = data.to_string();
                    
                    println!("{:#010x}: {:#010x}\t {}\t {}", 
                        (addr + (i * 4)).blue(), data.green(), self.disasm(data, (addr + i * 4).into()).magenta(), try_parse_string.red());
                }
            }
        }

        Ok(())
    }

    pub fn execute(&mut self, cmd: Cmds) -> ProcessResult<()> {
        match cmd {
            Cmds::Info { subcmd } => {
                self.cmd_info(subcmd)?;
            }

            _ => {
                Logger::todo();
            }
        }

        Ok(())
    }
}