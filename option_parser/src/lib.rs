use clap::Parser;

remu_macro::mod_flat!(config_parser, cli_parser, welcome);

pub fn parse() -> Result<(), ()> {
    config_parser().map_err(|e| {
        eprintln!("{}", e);
    })?;

    CLI::try_parse().map_err(|e| {
        let _ = e.print();
    })?;

    welcome();

    Ok(())
}
