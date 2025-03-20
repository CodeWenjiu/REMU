use std::result::Result;

use logger::Logger;

use crate::{cmd_parser::{Cmds, InfoCmds, MemoryCmds}, SimpleDebugger};

impl SimpleDebugger {
    fn cmd_info (&mut self, subcmd: InfoCmds) -> Result<(), ()> {
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

    fn cmd_memory (&mut self, subcmd: MemoryCmds) -> Result<(), ()> {
        match subcmd {
            MemoryCmds::ShowMemoryMap {} => {
                self.mmu.borrow().show_memory_map();
            }

            _ => {
                Logger::todo();
            }
        }

        Ok(())
    }

    pub fn execute(&mut self, cmd: Cmds) -> Result<(), ()> {
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