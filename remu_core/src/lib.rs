use clap::Parser;

remu_macro::mod_flat!(commands, error, command_expr);
pub use command_expr::{ExprParser, Rule};
pub use commands::get_command_graph;

pub struct Debugger {}

impl Debugger {
    pub fn new() -> Self {
        Debugger {}
    }

    pub fn execute_line(&self, buffer: String) -> Result<()> {
        let trimmed = buffer.trim();
        let expr =
            command_expr::parse_expression(trimmed).map_err(|_| Error::CommandExprHandled)?;

        // Parse all blocks up front; abort early on any invalid block
        let command_expr::CommandExpr { first, tail } = expr;

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
                (command_expr::Op::And, true) => {
                    result = self.execute_parsed(&cmd_wrapper.command)?;
                }
                (command_expr::Op::Or, false) => {
                    result = self.execute_parsed(&cmd_wrapper.command)?;
                }
                _ => {}
            }
        }
        let _ = result;
        Ok(())
    }

    fn parse_block(&self, mut tokens: Vec<String>) -> Result<CommandParser> {
        if tokens.is_empty() {
            return Err(Error::CommandExprHandled);
        }
        let mut commands = Vec::with_capacity(tokens.len() + 1);
        commands.push(env!("CARGO_PKG_NAME").to_string());
        commands.append(&mut tokens);

        match CommandParser::try_parse_from(commands) {
            Ok(v) => Ok(v),
            Err(e) => {
                let _ = e.print(); // keep clap colorized output
                Err(Error::CommandExprHandled)
            }
        }
    }

    fn execute_parsed(&self, command: &Commands) -> Result<bool> {
        match command {
            Commands::Version => {
                println!("remu-core v{}", env!("CARGO_PKG_VERSION"))
            }
            Commands::Continue => {
                tracing::info!("Continuing execution...");
            }
            Commands::Times { subcmd } => match subcmd {
                TimeCmds::Count { subcmd } => match subcmd {
                    TimeCountCmds::Test => {
                        tracing::info!("Time Count Test")
                    }
                },
            },
        }

        Ok(true)
    }
}
