use owo_colors::OwoColorize;
use remu_macro::{log_err, log_todo, log_warn};
use remu_utils::{ProcessError, ProcessResult};
use simulator::SimulatorItem;
use state::{mmu::Mask, reg::RegfileIo};

use crate::{cmd_parser::{BreakPointCmds, Cmds, DiffertestCmds, FunctionCmds, InfoCmds, MemorySetCmds, RegisterInfoCmds, RegisterSetCmds, SetCmds, StepCmds}, SimpleDebugger};

impl SimpleDebugger {
    fn cmd_info (&mut self, subcmd: InfoCmds) -> ProcessResult<()> {
        match subcmd {
            InfoCmds::Memory { subcmd } => {
                self.cmd_memory(subcmd)?;
            }

            InfoCmds::Register { subcmd } => {
                self.cmd_register_info(subcmd)?;
            }

            InfoCmds::Pipeline {  } => {
                println!("{}", self.state.pipe_state)
            }
        }

        Ok(())
    }

    fn cmd_memory (&mut self, subcmd: MemorySetCmds) -> ProcessResult<()> {
        match subcmd {
            MemorySetCmds::ShowMemoryMap {} => {
                self.state.mmu.show_memory_map();
            }

            MemorySetCmds::Examine { addr, length } => {
                for i in (0..(length * 4)).step_by(4) {
                    let i = i as u32;
                    let addr = self.eval_expr(&addr)?;
                    let data = log_err!(self.state.mmu.read_memory(addr + i, Mask::None), ProcessError::Recoverable)?;

                    print!("{:#010x}: {:#010x}\t",
                        (addr + i).blue(), data.green());
                    
                    self.conditional.try_analize(data, addr + i);

                    println!();
                }
            }
        }

        Ok(())
    }

    fn cmd_register_info (&mut self, subcmd: Option<RegisterInfoCmds>) -> ProcessResult<()> {
        match subcmd {
            Some(RegisterInfoCmds::CSR { index }) => {
                self.state.regfile.print_csr(index)?;
            }

            Some(RegisterInfoCmds::GPR { index }) => {
                self.state.regfile.print_gpr(index)?;
            }

            Some(RegisterInfoCmds::PC {}) => {
                self.state.regfile.print_pc();
            }

            None => {
                self.state.regfile.print_pc();
                self.state.regfile.print_gpr(None)?;
                self.state.regfile.print_csr(None)?;
            }
        }

        Ok(())
    }

    fn cmd_set (&mut self, subcmd: SetCmds) -> ProcessResult<()> {
        match subcmd {
            SetCmds::Register { subcmd } => {
                self.cmd_register_set(subcmd)?;
            }

            SetCmds::Memory { addr, value } => {
                let addr = self.eval_expr(&addr)?;
                let value = self.eval_expr(&value)?;
                log_err!(self.state.mmu.write(addr, value, Mask::None), ProcessError::Recoverable)?;
            }
        }

        log_warn!("Command `Set` is evil, carefully use it.");

        Ok(())
    }

    fn cmd_register_set (&mut self, subcmd: RegisterSetCmds) -> ProcessResult<()> {
        match subcmd {
            RegisterSetCmds::PC { value } => {
                let value = self.eval_expr(&value)?;
                self.state.regfile.set_pc(value)?;
            }

            RegisterSetCmds::GPR { index, value } => {
                let value = self.eval_expr(&value)?;
                self.state.regfile.set_gpr(index, value)?;
            }

            RegisterSetCmds::CSR { index, value } => {
                let value = self.eval_expr(&value)?;
                self.state.regfile.set_csr(index, value)?;
            }
        }

        Ok(())
    }

    fn cmd_breakpoint (&mut self, subcmd: BreakPointCmds) -> ProcessResult<()> {
        match subcmd {
            BreakPointCmds::Add { addr } => {
                let addr = self.eval_expr(&addr)?;
                self.simulator.tracer.borrow_mut().add_breakpoint(addr);
            }

            BreakPointCmds::Remove { addr } => {
                let addr = self.eval_expr(&addr)?;
                self.simulator.tracer.borrow_mut().remove_breakpoint_by_addr(addr);
            }

            BreakPointCmds::Show => {
                self.simulator.tracer.borrow_mut().show_breakpoints();
            }
        }

        Ok(())
    }

    fn cmd_step (&mut self, subcmd: StepCmds) -> ProcessResult<()> {
        match subcmd {
            StepCmds::Cycles { count } => {
                self.simulator.step_cycle(count)
            }

            StepCmds::Instructions { count } => {
                self.simulator.step_instruction(count)
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

    fn cmd_differtest (&mut self, subcmd: DiffertestCmds) -> ProcessResult<()> {
        match subcmd {
            DiffertestCmds::Info { subcmd } => {
                self.cmd_differtest_info(subcmd)?;
            }

            DiffertestCmds::Set { subcmd } => {
                self.cmd_differtest_set(subcmd)?;
            }

            DiffertestCmds::MemWatchPoint { addr } => {
                match addr {
                    Some(addr) => {
                        let addr = log_err!(self.eval_expr(&addr), ProcessError::Recoverable)?;
                        self.simulator.difftest_manager.as_ref().map(|man| {
                            man.borrow_mut().push_memory_watch_point(addr);
                        });
                    }

                    None => {
                        self.simulator.difftest_manager.as_ref().map(|man| {
                            man.borrow_mut().show_memory_watch_point();
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn cmd_differtest_info (&mut self, subcmd: InfoCmds) -> ProcessResult<()> {
        match subcmd {
            InfoCmds::Memory { subcmd } => {
                self.cmd_differtest_memory(subcmd)?;
            }

            InfoCmds::Register { subcmd } => {
                self.cmd_differtest_register(subcmd)?;
            }

            InfoCmds::Pipeline {  } => {
                println!("{}", self.state_ref.pipe_state)
            }
        }

        Ok(())
    }

    fn cmd_differtest_set (&mut self, subcmd: SetCmds) -> ProcessResult<()> {
        match subcmd {
            SetCmds::Register { subcmd } => {
                self.cmd_differtest_register_set(subcmd)?;
            }

            _ => {
                log_todo!();
            }
        }

        log_warn!("Command `Set` is evil, carefully use it.");

        Ok(())
    }

    fn cmd_differtest_register_set(&mut self, subcmd: RegisterSetCmds) -> ProcessResult<()> {
        match subcmd {
            RegisterSetCmds::GPR { index, value } => {
                let value = self.eval_expr(&value)?;
                self.state_ref.regfile.set_gpr(index, value)?;
            }

            _ => {
                log_todo!();
            }
        }

        Ok(())
    }

    fn cmd_differtest_memory (&mut self, subcmd: MemorySetCmds) -> ProcessResult<()> {
        match subcmd {
            MemorySetCmds::ShowMemoryMap {} => {
                self.state_ref.mmu.show_memory_map();
            }

            MemorySetCmds::Examine { addr, length } => {
                for i in (0..(length * 4)).step_by(4) {
                    let i = i as u32;
                    let addr = log_err!(self.eval_expr(&addr), ProcessError::Recoverable)?;
                    let data = log_err!(self.state_ref.mmu.read_memory(addr + i, Mask::None), ProcessError::Recoverable)?;

                    print!("{:#010x}: {:#010x}\t",
                        (addr + i).blue(), data.green());
                    
                    self.conditional.try_analize(data, addr + i);
                    
                    println!();
                }
            }
        }

        Ok(())
    }

    fn cmd_differtest_register (&mut self, subcmd: Option<RegisterInfoCmds>) -> ProcessResult<()> {
        match subcmd {
            Some(RegisterInfoCmds::CSR { index }) => {
                self.state.regfile.print_csr(index)?;
            }

            Some(RegisterInfoCmds::GPR { index }) => {
                self.state_ref.regfile.print_gpr(index)?;
            }

            Some(RegisterInfoCmds::PC {}) => {
                self.state_ref.regfile.print_pc();
            }

            None => {
                self.state_ref.regfile.print_pc();
                self.state_ref.regfile.print_gpr(None)?;
                self.state_ref.regfile.print_csr(None)?;
            }
        }

        Ok(())
    }

    pub fn execute(&mut self, cmd: Cmds) -> ProcessResult<()> {
        match cmd {
            Cmds::Step { subcmd } => {
                self.cmd_step(subcmd)?;
            }

            Cmds::Continue => {
                self.simulator.step_instruction(u64::MAX)?;
            }

            Cmds::Times => {
                self.simulator.dut.times()?;
            }

            Cmds::Info { subcmd } => {
                self.cmd_info(subcmd)?;
            }

            Cmds::Set { subcmd } => {
                self.cmd_set(subcmd)?;
            }

            Cmds::Print { expr } => {
                let result = self.eval_expr(&expr)?;
                println!("{} = {} : {:#08x}", expr.magenta(), result.green(), result.blue());
            }

            Cmds::BreakPoint { subcmd } => {
                self.cmd_breakpoint(subcmd)?;
            }

            Cmds::Function { subcmd } => {
                self.cmd_function(subcmd)?;
            }

            Cmds::Differtest { subcmd } => {
                self.cmd_differtest(subcmd)?;
            }
        }

        Ok(())
    }
}