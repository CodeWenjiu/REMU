use owo_colors::OwoColorize;
use remu_macro::{log_err, log_todo, log_warn};
use remu_utils::{ProcessError, ProcessResult};
use simulator::{difftest_ref::{AnyDifftestRef, DifftestRefPipelineApi}, SimulatorItem};
use state::{cache::{BtbData, CacheTrait}, mmu::Mask, reg::RegfileIo, States};

use crate::{cmd_parser::{BreakPointCmds, CacheCmds, Cmds, DiffertestCmds, FunctionCmds, InfoCmds, MemorySetCmds, RegisterInfoCmds, RegisterSetCmds, SetCmds, StepCmds, TestCmds}, SimpleDebugger};

#[derive(Clone, Copy)]
enum StateTarget {
    DUT,
    REF,
}

impl SimpleDebugger {
    fn get_state(&mut self, target: StateTarget) -> &mut States {
        match target {
            StateTarget::DUT => &mut self.state,
            StateTarget::REF => &mut self.state_ref,
        }
    }

    fn cmd_info (&mut self, subcmd: InfoCmds) -> ProcessResult<()> {
        match subcmd {
            InfoCmds::Memory { subcmd } => {
                self.cmd_info_memory(subcmd, StateTarget::DUT)?;
            }

            InfoCmds::Register { subcmd } => {
                self.cmd_info_register(subcmd, StateTarget::DUT)?;
            }

            InfoCmds::Pipeline {  } => {
                self.cmd_info_pipeline(StateTarget::DUT)?;
            }

            InfoCmds::Cache { subcmd } => {
                self.cmd_info_cache(subcmd, StateTarget::DUT)?;
            }

            InfoCmds::Extention { key } => {
                if let Some(key) = key {
                    self.simulator.dut.print_info(key.as_str());
                } else {
                    self.simulator.dut.get_keys().iter().for_each(|key| {
                        println!("- {}", key.blue());
                    });
                }
            }
        }

        Ok(())
    }

    fn cmd_set (&mut self, subcmd: SetCmds) -> ProcessResult<()> {
        match subcmd {
            SetCmds::Register { subcmd } => {
                self.cmd_register_set(subcmd, StateTarget::DUT)?;
            }

            SetCmds::Memory { addr, value } => {
                self.cmd_memory_set(&addr, &value, StateTarget::DUT)?;
            }

            SetCmds::Cache { set, way, tag, data } => {
                self.cmd_cache_set(set, way, tag, data, StateTarget::DUT)?;
            }
        }

        log_warn!("Command `Set` is evil, carefully use it.");

        Ok(())
    }

    fn cmd_differtest_info (&mut self, subcmd: InfoCmds) -> ProcessResult<()> {
        match subcmd {
            InfoCmds::Memory { subcmd } => {
                self.cmd_info_memory(subcmd, StateTarget::REF)?;
            }

            InfoCmds::Register { subcmd } => {
                self.cmd_info_register(subcmd, StateTarget::REF)?;
            }

            InfoCmds::Pipeline {  } => {
                self.cmd_info_pipeline(StateTarget::REF)?;
            }

            InfoCmds::Cache { subcmd } => {
                self.cmd_info_cache(subcmd, StateTarget::REF)?;
            }

            InfoCmds::Extention { key } => {
                if let Some(manager) = &self.simulator.difftest_manager {
                    let manager = &manager.borrow().reference;
                    let AnyDifftestRef::Pipeline(manager) = manager else {
                        log_todo!();
                        return Ok(());
                    };
                    
                    if let Some(key) = key {
                        manager.print_info(key.as_str());
                    } else {
                        manager.get_keys().iter().for_each(|key| {
                            println!("- {}", key.blue());
                        });
                    }
                } else {
                    log_todo!();
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    fn cmd_differtest_set (&mut self, subcmd: SetCmds) -> ProcessResult<()> {
        match subcmd {
            SetCmds::Register { subcmd } => {
                self.cmd_register_set(subcmd, StateTarget::REF)?;
            }

            SetCmds::Memory { addr, value } => {
                self.cmd_memory_set(&addr, &value, StateTarget::REF)?;
            }

            SetCmds::Cache { set, way, tag, data } => {
                self.cmd_cache_set(set, way, tag, data, StateTarget::REF)?;
            }
        }

        log_warn!("Command `Set` is evil, carefully use it.");

        Ok(())
    }

    fn cmd_differtest_test_cache(&mut self) -> ProcessResult<()> {
        match self.state.cache.btb.as_ref() {
            Some(btb) => {
                btb.test(&self.state_ref.cache.btb.as_ref().unwrap())?;
            }

            None => {
                log_warn!("BTB is not initialized, please check if the simulator supports it.");
            }
        };

        Ok(())
    }

    fn cmd_differtest_test(&mut self, subcmd: TestCmds) -> ProcessResult<()> {
        match subcmd {
            TestCmds::Cache {} => {
                self.cmd_differtest_test_cache()?;
            }
        }

        Ok(())
    }

    fn cmd_info_memory(&mut self, subcmd: MemorySetCmds, target: StateTarget) -> ProcessResult<()> {
        match subcmd {
            MemorySetCmds::ShowMemoryMap {} => {
                let target_state = self.get_state(target);
                target_state.mmu.show_memory_map()
            }

            MemorySetCmds::Examine { addr, length } => {
                let addr = self.eval_expr(&addr)?;
                
                for i in (0..(length * 4)).step_by(4) {
                    let i = i as u32;
                    let data = {
                        let target_state = self.get_state(target);
                        log_err!(target_state.mmu.read_memory(addr + i, Mask::None), ProcessError::Recoverable)?
                    };

                    print!("{:#010x}: {:#010x}\t",
                        (addr + i).blue(), data.green());
                    
                    self.conditional.try_analize(data, addr + i);

                    println!();
                }
            }
        }

        Ok(())
    }

    fn cmd_info_register(&mut self, subcmd: Option<RegisterInfoCmds>, target: StateTarget) -> ProcessResult<()> {
        let target_state = self.get_state(target);

        match subcmd {
            Some(RegisterInfoCmds::CSR { index }) => {
                log_err!(target_state.regfile.print_csr(index), ProcessError::Recoverable)?;
            }

            Some(RegisterInfoCmds::GPR { index }) => {
                target_state.regfile.print_gpr(index)?;
            }

            Some(RegisterInfoCmds::PC {}) => {
                target_state.regfile.print_pc();
            }

            None => {
                target_state.regfile.print_pc();
                target_state.regfile.print_gpr(None)?;
                log_err!(target_state.regfile.print_csr(None), ProcessError::Recoverable)?;
            }
        }

        Ok(())
    }

    fn cmd_info_pipeline(&mut self, target: StateTarget) -> ProcessResult<()> {
        let target_state = self.get_state(target);

        if let Some(pipe_state) = &target_state.pipe_state {
            println!("{}", pipe_state);
        } else {
            log_warn!("Pipeline state is not available, please check if the simulator supports it.");
        }

        Ok(())
    }

    fn cmd_info_cache(&mut self, subcmd: CacheCmds, target: StateTarget) -> ProcessResult<()> {
        let target_cache_state = &self.get_state(target).cache;

        match subcmd {
            CacheCmds::BTB => {
                if let Some(btb) = &target_cache_state.btb {
                    btb.print();
                } else {
                    log_warn!("BTB is not initialized, please check if the simulator supports it.");
                }
            }

            CacheCmds::ICache => {
                if let Some(icache) = &target_cache_state.icache {
                    icache.print();
                } else {
                    log_warn!("ICache is not initialized, please check if the simulator supports it.");
                }
            }
        }

        Ok(())
    }

    fn cmd_register_set (&mut self, subcmd: RegisterSetCmds, target: StateTarget) -> ProcessResult<()> {

        match subcmd {
            RegisterSetCmds::PC { value } => {
                let value = &self.eval_expr(&value)?;
                let target_state = self.get_state(target);
                target_state.regfile.set_pc(*value)?;
            }

            RegisterSetCmds::GPR { index, value } => {
                let value = &self.eval_expr(&value)?;
                let target_state = self.get_state(target);
                target_state.regfile.set_gpr(index, *value)?;
            }

            RegisterSetCmds::CSR { index, value } => {
                let value = &self.eval_expr(&value)?;
                let target_state = self.get_state(target);
                log_err!(target_state.regfile.set_csr(index, *value), ProcessError::Recoverable)?;
            }
        }

        Ok(())
    }

    fn cmd_memory_set(&mut self, addr: &str, value: &str, target: StateTarget) -> ProcessResult<()> {
        let addr = self.eval_expr(addr)?;
        let value = self.eval_expr(value)?;
        log_err!(self.get_state(target).mmu.write(addr, value, Mask::None), ProcessError::Recoverable)?;

        Ok(())
    }

    fn cmd_cache_set(&mut self, set: u32, way: u32, tag: u32, data: String, target: StateTarget) -> ProcessResult<()> {
        let data = &self.eval_expr(&data)?;
        
        let target_state = self.get_state(target);

        target_state.cache.btb.as_mut().map(|btb| {
            btb.base_write(set, way, 0, tag, BtbData{target: *data});
        });

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

            DiffertestCmds::Test { subcmd } => {
                self.cmd_differtest_test(subcmd)?;
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