use std::sync::Arc;

use clap::Parser;

remu_macro::mod_pub_flat!(prelude);
remu_macro::mod_pub_flat!(flow);
remu_macro::mod_flat!(error, compound_command);

use remu_harness::Harness;

pub struct Debugger<C: PlatformConfig> {
    harness: Harness<C>,
}

impl<C: PlatformConfig> Debugger<C> {
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

    pub fn execute_line(&mut self, buffer: String) -> Result<RunOutcome, DebuggerError> {
        let expr = compound_command::parse_expression(&buffer)?;
        self.execute_command_expr(&expr)
    }

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

        let mut outcome = self.execute_parsed(&first_cmd.command)?;
        for (_, cmd_wrapper) in tail.iter().zip(parsed_iter) {
            outcome = outcome.or_else(self.execute_parsed(&cmd_wrapper.command)?);
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
                let _ = e.print();
                Err(DebuggerError::CommandExprHandled)
            }
        }
    }

    fn execute_parsed(&mut self, command: &Command) -> Result<RunOutcome, DebuggerError> {
        match command {
            Command::Step { times } => self
                .harness
                .run_steps(Some(*times))
                .map_err(DebuggerError::CommandExec),
            Command::Continue => self
                .harness
                .run_steps(None)
                .map_err(DebuggerError::CommandExec),
            Command::Func { subcmd } => {
                self.harness.func_exec(subcmd);
                Ok(RunOutcome::Done)
            }
            Command::State { subcmd } => self
                .harness
                .state_exec(subcmd)
                .map_err(DebuggerError::CommandExec)
                .map(|()| RunOutcome::Done),
            Command::RefState { subcmd } => self
                .harness
                .ref_state_exec(subcmd)
                .map_err(DebuggerError::CommandExec)
                .map(|()| RunOutcome::Done),
            Command::Breakpoint { subcmd } => match subcmd {
                BreakpointCmd::Set { addr } => self
                    .harness
                    .set_breakpoint(*addr)
                    .map_err(DebuggerError::CommandExec)
                    .map(|()| RunOutcome::Done),
                BreakpointCmd::Del { addr } => self
                    .harness
                    .del_breakpoint(*addr)
                    .map_err(DebuggerError::CommandExec)
                    .map(|()| RunOutcome::Done),
                BreakpointCmd::Print => {
                    self.harness.print_breakpoints();
                    Ok(RunOutcome::Done)
                }
            },
            Command::Stat { subcmd } => {
                self.harness.stat_exec(subcmd);
                Ok(RunOutcome::Done)
            }
            Command::Quit => Err(DebuggerError::ExitRequested),
        }
    }
}
