use clap::Parser;

remu_macro::mod_flat!(config_parser, cli_parser, welcome);

pub struct OptionParser {
    pub cfg: Cfg,
    pub cli: cli_parser::CLI,
}

pub fn parse() -> Result<OptionParser, ()> {
    let cli = CLI::try_parse().map_err(|e| {
        let _ = e.print();
    })?;

    let cfg = if let Some(config_path) = &cli.config {
        parse_config(config_path.into(), &cli.platform)?
    } else {
        Cfg::default()
    };

    welcome(&cli.platform);

    Ok(OptionParser { cfg, cli })
}
