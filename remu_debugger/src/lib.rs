use std::sync::Arc;

use clap::Parser;

remu_macro::mod_flat!(command, option, policy, error, compound_command);
pub use command::get_command_graph;
pub use compound_command::{CommandExpr, Op, ParseError};
use remu_harness::{DutSim, Harness};
pub use remu_harness::{ExitCode, RunOutcome};
use remu_types::TracerDyn;

pub struct Debugger<P: HarnessPolicy, R: SimulatorTrait<P, false>> {
    harness: Harness<DutSim<P>, R>,
}

impl<P: HarnessPolicy, R: SimulatorTrait<P, false>> Debugger<P, R> {
    pub fn new(
        opt: DebuggerOption,
        tracer: TracerDyn,
        interrupt: Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            harness: Harness::new(opt.sim, tracer, interrupt),
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
        self.execute_command_expr(&startup).map(drop)
    }

    /// Returns the run outcome (Done or ProgramExit(code)) for the last run, if any.
    pub fn execute_line(&mut self, buffer: String) -> Result<RunOutcome, DebuggerError> {
        let expr = compound_command::parse_expression(&buffer)?;
        self.execute_command_expr(&expr)
    }

    /// Execute a pre-parsed command expression (e.g. from startup sequence).
    /// Returns the merged run outcome (ProgramExit wins over Done when multiple commands run).
    pub fn execute_command_expr(
        &mut self,
        expr: &CommandExpr,
    ) -> Result<RunOutcome, DebuggerError> {
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
            None => return Ok(RunOutcome::Done),
        };

        let (mut result, mut outcome) = self.execute_parsed(&first_cmd.command)?;
        for ((op, _), cmd_wrapper) in tail.iter().zip(parsed_iter) {
            match (*op, result) {
                (compound_command::Op::And, true) => {
                    let (r, out) = self.execute_parsed(&cmd_wrapper.command)?;
                    result = r;
                    outcome = outcome.or_else(out);
                }
                (compound_command::Op::Or, false) => {
                    let (r, out) = self.execute_parsed(&cmd_wrapper.command)?;
                    result = r;
                    outcome = outcome.or_else(out);
                }
                _ => {}
            }
        }
        Ok(outcome)
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

    fn execute_parsed(&mut self, command: &Command) -> Result<(bool, RunOutcome), DebuggerError> {
        match command {
            Command::Step { times } => {
                let outcome = self
                    .harness
                    .run_steps(Some(*times))
                    .map_err(DebuggerError::CommandExec)?;
                return Ok((true, outcome));
            }
            Command::Continue => {
                let outcome = self
                    .harness
                    .run_steps(None)
                    .map_err(DebuggerError::CommandExec)?;
                return Ok((true, outcome));
            }
            Command::Func { subcmd } => {
                self.harness.func_exec(subcmd);
            }
            Command::State { subcmd } => {
                if let Err(e) = self.harness.state_exec(subcmd) {
                    eprintln!("{}", e);
                    return Ok((false, RunOutcome::Done));
                }
            }
            Command::Quit => return Err(DebuggerError::ExitRequested),
        }
        Ok((true, RunOutcome::Done))
    }
}
