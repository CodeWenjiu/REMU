use std::io::IsTerminal;
use std::sync::Arc;

use clap::Parser;

remu_macro::mod_pub!(prelude);
remu_macro::mod_pub!(crate, flow);
remu_macro::mod_prv!(error, compound_command);

pub use compound_command::{CommandExpr, Op, ParseError, parse_expression, startup_to_expr};
pub use error::DebuggerError;
pub use flow::command::{BreakpointCmd, Command, DebuggerCommand, get_command_graph};
pub use flow::generic::DebuggerRunner;
pub use flow::option::DebuggerOption;

pub use remu_harness::ErrorStyle;
use remu_harness::Harness;
use remu_harness::StatCmd;
pub use remu_harness::{PlatformConfig, RunOutcome};
pub use remu_types::ExitCode;
use remu_types::TracerDyn;

/// What the CLI should do after running startup commands.
pub enum StartupDecision {
    /// Enter the interactive REPL (non-batch mode).
    EnterRepl,
    /// Leave the run (batch mode), reporting what the startup left behind so the
    /// frontend can turn a failing program exit into a non-zero status.
    Exit(RunOutcome),
    /// Exit with failure (batch mode, startup command failed; detail already printed).
    ExitErr,
}

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

    /// Run startup commands. The returned [`StartupDecision`] tells the caller
    /// whether to enter the REPL or exit (and with which exit code).
    /// Error detail is consumed (printed) inside this method; callers only see
    /// the decision.
    pub fn run_startup(&mut self, opt: &DebuggerOption) -> StartupDecision {
        let result = crate::compound_command::startup_to_expr(&opt.startup)
            .map_err(DebuggerError::from)
            .and_then(|expr| self.execute_command_expr(&expr));
        if matches!(result, Err(DebuggerError::ExitRequested)) {
            return StartupDecision::Exit(RunOutcome::Done);
        }
        let batch_decision = match &result {
            Ok(outcome) => StartupDecision::Exit(*outcome),
            Err(e) => {
                eprintln!("startup execution error: {}", e);
                StartupDecision::ExitErr
            }
        };
        if opt.batch {
            batch_decision
        } else {
            StartupDecision::EnterRepl
        }
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

        let mut commands = Vec::new();
        for block in std::iter::once(first).chain(tail.iter().map(|(_, b)| b)) {
            if block.is_empty() {
                continue;
            }
            if let BlockAction::Run(cmd) = self.parse_block(block)? {
                commands.push(cmd);
            }
        }

        let mut outcome = RunOutcome::Done;
        for cmd in &commands {
            outcome = outcome.or_else(self.execute_parsed(&cmd.command)?);
        }
        Ok(outcome)
    }

    fn parse_block(&self, tokens: &[String]) -> Result<BlockAction, DebuggerError> {
        let mut argv = Vec::with_capacity(tokens.len() + 1);
        argv.push(env!("CARGO_PKG_NAME").to_string());
        argv.extend_from_slice(tokens);

        match DebuggerCommand::try_parse_from(argv) {
            Ok(v) => Ok(BlockAction::Run(v)),
            Err(e) if is_display_request(&e) => {
                let _ = e.print();
                Ok(BlockAction::Displayed)
            }
            Err(e) => Err(DebuggerError::CommandParse(render_clap_error(&e))),
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
            Command::Stat { query } => self
                .harness
                .stat_exec(&StatCmd::from_query(query.as_deref()))
                .map_err(DebuggerError::CommandExec)
                .map(|()| RunOutcome::Done),
            Command::Quit => Err(DebuggerError::ExitRequested),
        }
    }
}

enum BlockAction {
    Run(DebuggerCommand),
    Displayed,
}

fn is_display_request(e: &clap::Error) -> bool {
    use clap::error::ErrorKind::*;
    matches!(
        e.kind(),
        DisplayHelp | DisplayHelpOnMissingArgumentOrSubcommand | DisplayVersion
    )
}

fn render_clap_error(e: &clap::Error) -> String {
    let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
    let rendered = if !no_color && std::io::stderr().is_terminal() {
        e.render().ansi().to_string()
    } else {
        e.render().to_string()
    };
    rendered.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_is_a_display_request_not_a_failure() {
        let e = DebuggerCommand::try_parse_from(["remu_debugger", "help"]).unwrap_err();
        assert!(is_display_request(&e), "{e}");
    }

    #[test]
    fn unknown_command_is_a_real_failure() {
        let e = DebuggerCommand::try_parse_from(["remu_debugger", "frobnicate"]).unwrap_err();
        assert!(!is_display_request(&e), "{e}");
    }
}
