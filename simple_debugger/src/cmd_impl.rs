use logger::Logger;
use state::mmu::Mask;

use crate::{cmd_parser::{Cmds, InfoCmds, MemoryCmds}, ProcessError, ProcessResult, SimpleDebugger};

impl SimpleDebugger {
    fn cmd_info (&mut self, subcmd: InfoCmds) -> ProcessResult<()> {
        match subcmd {
            InfoCmds::Memory { subcmd } => {
                self.cmd_memory(subcmd)?;
            }
            
            _ => {
                Logger::todo();
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
                    
                    Logger::show(
                        &format!("{:#010x}: {:#010x} {}", 
                        addr + (i * 4), data, self.disasm(data, (addr + i * 4).into())), 
                        Logger::INFO
                    );
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