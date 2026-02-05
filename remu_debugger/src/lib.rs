use clap::Parser;

remu_macro::mod_flat!(command, option, policy, error, compound_command, run_state);
pub use command::get_command_graph;
use remu_harness::{DutSim, Harness, SimulatorError, SimulatorInnerError};
use remu_types::TracerDyn;

pub struct Debugger<P: HarnessPolicy, R: SimulatorTrait<P, false>> {
    harness: Harness<DutSim<P>, R>,
    run_state: RunState,
}

impl<P: HarnessPolicy, R: SimulatorTrait<P, false>> Debugger<P, R> {
    pub fn new(opt: DebuggerOption, tracer: TracerDyn) -> Self {
        Debugger {
            harness: Harness::new(opt.sim, tracer),
            run_state: RunState::Idle,
        }
    }

    pub fn execute_line(&mut self, buffer: String) -> Result<()> {
        let trimmed = buffer.trim();

        // If the command expression itself is invalid (e.g. bad braces like "{]"),
        // surface that parse error to the user instead of swallowing it and later
        // falling back to clap's "unrecognized subcommand".
        let expr = compound_command::parse_expression(trimmed)?;

        // Parse all blocks up front; abort early on any invalid block
        let compound_command::CommandExpr { first, tail } = expr;

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
        for ((op, _), cmd_wrapper) in tail.into_iter().zip(parsed_iter) {
            match (op, result) {
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

    fn parse_block(&self, mut tokens: Vec<String>) -> Result<DebuggerCommand> {
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

    fn execute_parsed(&mut self, command: &Command) -> Result<bool> {
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
        }
        Ok(true)
    }

    fn run_step_loop(&mut self, max_steps: Option<usize>) -> Result<()> {
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
                if let SimulatorError::Dut(SimulatorInnerError::ProgramExit(code)) = e {
                    self.run_state = RunState::Exit;
                    return Err(DebuggerError::ProgramExit(code));
                }
                return Err(DebuggerError::CommandExec(e));
            }
            steps += 1;
        }
    }
}
