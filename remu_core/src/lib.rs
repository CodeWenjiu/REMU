use clap::Parser;

remu_macro::mod_flat!(commands, error, command_expr);
pub use commands::get_command_graph;

pub struct Debugger {}

impl Debugger {
    pub fn new() -> Self {
        Debugger {}
    }

    pub fn execute_line(&self, buffer: String) -> Result<()> {
        let trimmed = buffer.trim();

        // Try full expression (do {...} and/or do {...}) first
        match command_expr::parse_expression(trimmed) {
            Ok(expr) => {
                let mut result = self.execute_tokens(expr.first)?;
                for (op, block) in expr.tail {
                    match op {
                        command_expr::Op::And => {
                            if result {
                                result = self.execute_tokens(block)?;
                            }
                        }
                        command_expr::Op::Or => {
                            if !result {
                                result = self.execute_tokens(block)?;
                            }
                        }
                    }
                }
                let _ = result;
                return Ok(());
            }
            Err(e) => {
                if trimmed.starts_with("do") {
                    return Err(Error::CommandExpr(e.to_string()));
                }
            }
        }

        // Fallback: treat as a single command
        let tokens = match shlex::split(trimmed) {
            Some(v) if !v.is_empty() => v,
            _ => return Ok(()),
        };
        let _ = self.execute_tokens(tokens)?;
        Ok(())
    }

    fn execute_tokens(&self, mut tokens: Vec<String>) -> Result<bool> {
        if tokens.is_empty() {
            return Ok(true);
        }
        let mut commands = Vec::with_capacity(tokens.len() + 1);
        commands.push(env!("CARGO_PKG_NAME").to_string());
        commands.append(&mut tokens);

        let cmd_wrapper = match CommandParser::try_parse_from(commands) {
            Ok(v) => v,
            Err(e) => {
                let _ = e.print();
                return Ok(false);
            }
        };

        match cmd_wrapper.command {
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
