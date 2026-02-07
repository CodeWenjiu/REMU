use clap::Parser;

remu_macro::mod_flat!(command, option, policy, error, compound_command, run_state);
pub use command::get_command_graph;
pub use compound_command::{CommandExpr, Op, ParseError};
use remu_harness::{DutSim, Harness, SimulatorError, SimulatorInnerError};
use remu_types::TracerDyn;

pub struct Debugger<P: HarnessPolicy, R: SimulatorTrait<P, false>> {
    harness: Harness<DutSim<P>, R>,
    run_state: RunState,
}

impl<P: HarnessPolicy, R: SimulatorTrait<P, false>> Debugger<P, R> {
    pub fn new(opt: DebuggerOption, tracer: TracerDyn) -> Result<Self, DebuggerError> {
        let mut debugger = Debugger {
            harness: Harness::new(opt.sim, tracer),
            run_state: RunState::Idle,
        };

        let startup_tokens = opt.startup.as_slice();
        let expr = crate::compound_command::startup_to_expr(startup_tokens);
        let startup = if opt.batch {
            expr.with_continue_prepended().with_quit_appended()
        } else {
            expr
        };
        debugger.execute_command_expr(&startup)?;

        Ok(debugger)
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
            Command::RefState { subcmd } => {
                if let Err(e) = self.harness.ref_state_exec(subcmd) {
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
            return Err(DebuggerError::ExitRequested);
        }

        let mut steps = 0usize;
        loop {
            if let Some(limit) = max_steps {
                if steps >= limit {
                    return Ok(());
                }
            }
            if let Err(e) = self.harness.step_once() {
                if let SimulatorError::Dut(inner) = &e {
                    if let SimulatorInnerError::ProgramExit(code) = inner {
                        self.run_state = RunState::Exit;
                        return Err(DebuggerError::ProgramExit(*code));
                    }
                }
                return Err(DebuggerError::CommandExec(e));
            }
            steps += 1;
        }
    }
}
