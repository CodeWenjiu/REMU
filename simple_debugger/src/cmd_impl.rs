use logger::Logger;
use owo_colors::OwoColorize;
use remu_macro::log_err;
use remu_utils::{ProcessError, ProcessResult};
use state::mmu::Mask;

use crate::{cmd_parser::{Cmds, FunctionCmds, InfoCmds, MemoryCmds, RegisterCmds, StepCmds}, SimpleDebugger};

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
                self.state.regfile.borrow().print_csr(index);
            }

            Some(RegisterCmds::GPR { index }) => {
                self.state.regfile.borrow().print_gpr(index);
            }

            Some(RegisterCmds::PC {}) => {
                self.state.regfile.borrow().print_pc();
            }

            None => {
                self.state.regfile.borrow().print_pc();
                self.state.regfile.borrow().print_gpr(None);
                self.state.regfile.borrow().print_csr(None);
            }
        }

        Ok(())
    }

    fn cmd_memory (&mut self, subcmd: MemoryCmds) -> ProcessResult<()> {
        match subcmd {
            MemoryCmds::ShowMemoryMap {} => {
                self.state.mmu.show_memory_map();
            }

            MemoryCmds::Examine { addr, length } => {
                for i in (0..(length * 4)).step_by(4) {
                    let i = i as u32;
                    let data = log_err!(self.state.mmu.read(addr + i, Mask::None), ProcessError::Recoverable)?;

                    println!("{:#010x}: {:#010x}\t {}",
                        (addr + i).blue(), data.green(), self.disassembler.borrow().try_analize(data, addr + i).magenta());
                }
            }
        }

        Ok(())
    }

    fn cmd_step (&mut self, subcmd: StepCmds) -> ProcessResult<()> {
        match subcmd {
            StepCmds::Cycles { count } => {
                for _ in 0..count {
                    self.simulator.as_mut().step_cycle()?
                }
                Ok(())
            }

            StepCmds::Instructions { count } => {
                Logger::todo();
                let _ = count;
                Ok(())
            }
        }
    }

    fn cmd_function (&mut self, subcmd: FunctionCmds) -> ProcessResult<()> {
        match subcmd {
            FunctionCmds::Disable { subcmd } => {
                self.simulator.cmd_function_mut(subcmd, false)?;
            }

            FunctionCmds::Enable { subcmd } => {
                self.simulator.cmd_function_mut(subcmd, true)?;
            }
        }

        Ok(())
    }

    pub fn execute(&mut self, cmd: Cmds) -> ProcessResult<()> {
        match cmd {
            Cmds::Step { subcmd } => {
                self.cmd_step(subcmd)?;
            }

            Cmds::Info { subcmd } => {
                self.cmd_info(subcmd)?;
            }

            Cmds::Function { subcmd } => {
                self.cmd_function(subcmd)?;
            }

            _ => {
                Logger::todo();
            }
        }

        Ok(())
    }
}