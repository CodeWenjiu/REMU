use std::collections::HashMap;

use clap::Parser;
use logger::Logger;

remu_macro::mod_flat!(config_parser, cli_parser, welcome);

// #[derive(Debug, snafu::Snafu)]
// pub enum ConfigError {
//     #[snafu(display("{} {}: {}", "Unable to parse config file from".red(), path.display(), source))]
//     PlatformSyntax { source: config::ConfigError, path: PathBuf },
// }

pub struct OptionParser {
    pub config: HashMap<String, String>,
    pub cli: cli_parser::CLI,
}

pub fn parse() -> Result<OptionParser, ()> {
    let mut config = config_parser().map_err(|e| {
        Logger::show(&e.to_string(), Logger::ERROR);
    })?;

    let cli = CLI::try_parse().map_err(|e| {
        let _ = e.print();
    })?;

    let (arch, emu_platorm) = cli.platform.split_once("-").unwrap();

    config = config
        .iter()
        .filter(|s| s.0.starts_with(emu_platorm))
        .map(|(k, v)| (k.replace(emu_platorm, arch), v.clone()))
        .collect();

    println!("{:?}", config);

    let mut regions: HashMap<(String, String), (Option<u32>, Option<u32>)> = HashMap::new();

    fn parse_hex(s: &str) -> Result<u32, ()> {
        let s = s.trim_start_matches("0x");
        u32::from_str_radix(s, 16).map_err(|_| ())
    }

    for (key, value) in config.iter() {
        let parts: Vec<&str> = key.split('_').collect();
        if parts.len() != 4 {
            Logger::show(&format!("Invalid platform syntax: {}", key), Logger::ERROR);
            return Err(());
        }

        let isa = parts[0];
        let name = parts[2];
        let attr = parts[3];

        if attr != "BASE" && attr != "SIZE" {
            Logger::show(&format!("Invalid platform syntax: {}", key), Logger::ERROR);
            return Err(());
        }

        let value = parse_hex(value)?;

        let entry = regions
            .entry((isa.to_string(), name.to_string()))
            .or_insert((None, None));
        match attr {
            "BASE" => entry.0 = Some(value),
            "SIZE" => entry.1 = Some(value),
            _ => unreachable!(),
        }
    }

    // unwarp regions
    let regions = regions
        .iter()
        .map(|((isa, name), (base, size))| {
            (
                isa,
                name,
                base.unwrap_or_default(),
                size.unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();

    println!("{:?}", regions);

    welcome(&cli.platform);

    Ok(OptionParser { config, cli })
}
