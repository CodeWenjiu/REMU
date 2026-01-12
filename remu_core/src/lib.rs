use clap::{CommandFactory, Parser};

remu_macro::mod_flat!(commands, error);

fn get_command_list() -> Vec<String> {
    let command = CommandParser::command();
    command
        .get_subcommands()
        .map(|sub| sub.get_name().to_string())
        .collect()
}

pub fn get_command_with_help() -> Vec<String> {
    let mut commands = get_command_list();
    commands.push("help".to_string());
    commands
}

pub struct Debugger {}

impl Debugger {
    pub fn new() -> Self {
        Debugger {}
    }

    pub fn execute(&self, buffer: String) -> Result<()> {
        let mut commands = shlex::split(&buffer).ok_or(Error::InvalidQuoting)?;

        commands.insert(0, "remu_core".to_string());

        let cmd_wrapper = match CommandParser::try_parse_from(commands) {
            Ok(v) => v,
            Err(e) => {
                let _ = e.print();
                return Ok(());
            }
        };

        match cmd_wrapper.command {
            Commands::Continue => {
                println!("Continuing execution...");
            }
            Commands::Times { count } => {
                println!("Executing command {} times...", count);
            }
        }

        Ok(())
    }
}
