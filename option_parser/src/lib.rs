use clap::Parser;

remu_macro::mod_flat!(config_parser, cli_parser, welcome);

pub struct OptionParser {
    pub cfg: CfgAf,
    pub cli: cli_parser::CLI,
}

pub fn parse() -> Result<OptionParser, ()> {
    let cli = CLI::try_parse().map_err(|e| {
        let _ = e.print();
    })?;

    let cfg = parse_config("config/.config".into(), &cli.platform)?;

    welcome(&cli.platform);

    Ok(OptionParser { cfg, cli })
}
