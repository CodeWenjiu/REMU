use clap::Parser;

remu_macro::mod_flat!(commands, error);
pub use commands::get_command_graph;

pub struct Debugger {}

impl Debugger {
    pub fn new() -> Self {
        Debugger {}
    }

    pub fn execute(&self, buffer: String) -> Result<()> {
        let mut commands = shlex::split(&buffer).ok_or(Error::InvalidQuoting)?;

        commands.insert(0, env!("CARGO_PKG_NAME").to_string());

        let cmd_wrapper = match CommandParser::try_parse_from(commands) {
            Ok(v) => v,
            Err(e) => {
                let _ = e.print();
                return Ok(());
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

        Ok(())
    }
}
