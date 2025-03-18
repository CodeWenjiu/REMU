use std::collections::HashMap;

use clap::Parser;

remu_macro::mod_flat!(config_parser, cli_parser, welcome);

pub struct OptionParser {
    pub config: HashMap<String, String>,
    pub cli: cli_parser::CLI,
}

pub fn parse() -> Result<OptionParser, ()> {
    let config = config_parser().map_err(|e| {
        eprintln!("{}", e);
    })?;

    let cli = CLI::try_parse().map_err(|e| {
        let _ = e.print();
    })?;

    welcome();

    Ok(OptionParser { config, cli })
}
