use std::sync::Arc;
use std::sync::atomic::Ordering;

use clap::Parser;

remu_macro::mod_flat!(command, option, policy, error, compound_command, run_state);
pub use command::get_command_graph;
pub use compound_command::{CommandExpr, Op, ParseError};
use remu_harness::{DutSim, Harness, SimulatorError, SimulatorInnerError};
use remu_types::TracerDyn;

pub struct Debugger<P: HarnessPolicy, R: SimulatorTrait<P, false>> {
    harness: Harness<DutSim<P>, R>,
    run_state: RunState,
    interrupt: Arc<std::sync::atomic::AtomicBool>,
}

impl<P: HarnessPolicy, R: SimulatorTrait<P, false>> Debugger<P, R> {
    pub fn new(
        opt: DebuggerOption,
        tracer: TracerDyn,
        interrupt: Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            harness: Harness::new(opt.sim, tracer),
            run_state: RunState::Idle,
            interrupt,
        }
    }

    pub fn run_startup(&mut self, opt: &DebuggerOption) -> Result<(), DebuggerError> {
        let startup_tokens = opt.startup.as_slice();
        let expr = crate::compound_command::startup_to_expr(startup_tokens);
        let startup = if opt.batch {
            expr.with_continue_prepended().with_quit_appended()
        } else {
            expr
        };
        self.execute_command_expr(&startup)
    }

    pub fn execute_line(&mut self, buffer: String) -> Result<(), DebuggerError> {
        let expr = compound_command::parse_expression(&buffer)?;
        self.execute_command_expr(&expr)
    }

    /// Execute a pre-parsed command expression (e.g. from startup sequence).
    pub fn execute_command_expr(&mut self, expr: &CommandExpr) -> Result<(), DebuggerError> {
        let CommandExpr { first, tail } = expr;

        let blocks_iter = std::iter::once(first.clone()).chain(tail.iter().map(|(_, b)| b.clone()));

        let mut parsed = Vec::new();
        for block in blocks_iter {
            if block.is_empty() {
                continue;
            }
            parsed.push(self.parse_block(block)?);
        }

        let mut parsed_iter = parsed.into_iter();
        let first_cmd = match parsed_iter.next() {
            Some(cmd) => cmd,
            None => return Ok(()),
        };

        let mut result = self.execute_parsed(&first_cmd.command)?;
        for ((op, _), cmd_wrapper) in tail.iter().zip(parsed_iter) {
            match (*op, result) {
                (compound_command::Op::And, true) => {
                    result = self.execute_parsed(&cmd_wrapper.command)?;
                }
                (compound_command::Op::Or, false) => {
                    result = self.execute_parsed(&cmd_wrapper.command)?;
                }
                _ => {}
            }
        }
        let _ = result;
        Ok(())
    }

    fn parse_block(&self, mut tokens: Vec<String>) -> Result<DebuggerCommand, DebuggerError> {
        let mut commands = Vec::with_capacity(tokens.len() + 1);
        commands.push(env!("CARGO_PKG_NAME").to_string());
        commands.append(&mut tokens);

        match DebuggerCommand::try_parse_from(commands) {
            Ok(v) => Ok(v),
            Err(e) => {
                let _ = e.print(); // keep clap colorized output
                Err(DebuggerError::CommandExprHandled)
            }
        }
    }

    fn execute_parsed(&mut self, command: &Command) -> Result<bool, DebuggerError> {
        match command {
            Command::Step { times } => {
                self.run_step_loop(Some(*times))?;
            }
            Command::Continue => {
                self.run_step_loop(None)?;
            }
            Command::Func { subcmd } => {
                self.harness.func_exec(subcmd);
            }
            Command::State { subcmd } => {
                if let Err(e) = self.harness.state_exec(subcmd) {
                    eprintln!("{}", e);
                    return Ok(false);
                }
            }
            Command::Quit => return Err(DebuggerError::ExitRequested),
        }
        Ok(true)
    }

    fn run_step_loop(&mut self, max_steps: Option<usize>) -> Result<(), DebuggerError> {
        if self.run_state == RunState::Exit {
            return Ok(());
        }

        const BATCH: usize = 1024;
        let mut steps = 0usize;
        loop {
            if let Some(limit) = max_steps {
                if steps >= limit {
                    return Ok(());
                }
            }
            if self.interrupt.load(Ordering::Relaxed) {
                self.interrupt.store(false, Ordering::Relaxed);
                return Err(DebuggerError::Interrupted);
            }
            let to_run = max_steps
                .map(|limit| (limit - steps).min(BATCH))
                .unwrap_or(BATCH);
            match self.harness.step_n(to_run) {
                Ok(k) => steps += k,
                Err(e) => {
                    if let SimulatorError::Dut(inner) = &e {
                        if let SimulatorInnerError::ProgramExit(_code) = inner {
                            self.run_state = RunState::Exit;
                            return Ok(());
                        }
                    }
                    return Err(DebuggerError::CommandExec(e));
                }
            }
        }
    }
}
