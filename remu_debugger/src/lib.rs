use clap::Parser;

remu_macro::mod_flat!(command, option, policy, error, compound_command);
pub use command::get_command_graph;
use remu_simulator::Simulator;
use remu_types::TracerDyn;

pub struct Debugger<P: DebuggerPolicy> {
    simulator: Simulator<P>,
}

impl<P: DebuggerPolicy> Debugger<P> {
    pub fn new(opt: DebuggerOption, tracer: TracerDyn) -> Self {
        Debugger {
            simulator: Simulator::new(opt.sim, tracer),
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

        if parsed.is_empty() {
            return Ok(());
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
                for _ in 0..*times {
                    self.simulator.step_once()?;
                }
            }
            Command::Continue => {
                for _ in 0..usize::MAX {
                    self.simulator.step_once()?;
                }
            }
            Command::Func { subcmd } => {
                self.simulator.func_exec(subcmd);
            }
            Command::State { subcmd } => {
                if let Err(e) = self.simulator.state_exec(subcmd) {
                    eprintln!("{}", e);
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}
