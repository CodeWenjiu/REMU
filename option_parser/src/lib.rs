use std::collections::HashMap;

use clap::Parser;
use logger::Logger;
use state::mmu::MemoryFlags;

remu_macro::mod_flat!(config_parser, cli_parser, welcome);

pub struct OptionParser {
    pub memory_config: Vec<(String, String, u32, u32, MemoryFlags)>,
    pub debug_config: Vec<DebugConfiguration>,
    pub cli: cli_parser::CLI,
}

fn parse_hex(s: &str) -> Result<u32, ()> {
    let s = s.trim_start_matches("0x");
    u32::from_str_radix(s, 16).map_err(|e| 
        Logger::show(&e.to_string(), Logger::ERROR)
    )
}

fn parse_bin(s: &str) -> Result<u32, ()> {
    let s = s.trim_start_matches("0b");
    u32::from_str_radix(s, 2).map_err(|e| 
        Logger::show(&e.to_string(), Logger::ERROR)
    )
}

fn parse_dec(s: &str) -> Result<u32, ()> {
    u32::from_str_radix(s, 10).map_err(|e| 
        Logger::show(&e.to_string(), Logger::ERROR)
    )
}

fn parse_memory_region(config: &HashMap<String, String>, platform: &str) -> Result<Vec<(String, String, u32, u32, MemoryFlags)>, ()> {
    let (arch, emu_platorm) = platform.split_once("-").unwrap();
    let arch = arch.to_uppercase();
    let emu_platorm = emu_platorm.to_uppercase();

    let mem_config: Vec<(String, String)> = config
        .iter()
        .filter(|s| s.0.starts_with(&emu_platorm))
        .map(|(k, v)| (k.replace(&emu_platorm, &arch), v.clone()))
        .collect();

    let mut regions: HashMap<(String, String), (Option<u32>, Option<u32>, Option<MemoryFlags>)> = HashMap::new();

    for (key, value) in mem_config.iter() {
        let parts: Vec<&str> = key.split('_').collect();
        if parts.len() != 4 {
            Logger::show(&format!("Invalid platform syntax: {}", key), Logger::ERROR);
            return Err(());
        }

        let isa = parts[0];
        let name = parts[2];
        let attr = parts[3];

        if attr != "BASE" && attr != "SIZE" && attr != "FLAG" {
            Logger::show(&format!("Invalid platform syntax: {}", key), Logger::ERROR);
            return Err(());
        }

        let entry = regions
            .entry((isa.to_string(), name.to_string()))
            .or_insert((None, None, None));
        match attr {
            "BASE" => entry.0 = Some(parse_hex(value)?),
            "SIZE" => entry.1 = Some(parse_hex(value)?),
            "FLAG" => entry.2 = Some(MemoryFlags::from_bits(parse_bin(value)?.try_into().unwrap()).unwrap()),
            _ => unreachable!(),
        }
    }

    // unwarp regions
    let regions: Vec<(String, String, u32, u32, MemoryFlags)> = regions
        .iter()
        .map(|((isa, name), (base, size, flag))| {
            (
                isa.clone(),
                name.clone(),
                base.unwrap(),
                size.unwrap(),
                flag.clone().unwrap(),
            )
        })
        .collect::<Vec<_>>();

    Ok(regions)
}

#[derive(Debug)]
pub enum DebugConfiguration {
    Readline {
        history: u32,
    }
}

fn parse_debug_configuration(config: &HashMap<String, String>) -> Result<Vec<DebugConfiguration>, ()> {
    let debug_config: Vec<(String, String)> = config
        .iter()
        .filter(|s| s.0.starts_with("DEBUG"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    for (key, value) in debug_config.iter() {
        let parts: Vec<&str> = key.split('_').collect();
        if parts.len() != 4 {
            Logger::show(&format!("Invalid debug syntax: {}", key), Logger::ERROR);
            return Err(());
        }

        let attr = parts[1];
        if attr == "RL" {
            let history = parse_dec(value)?;
            return Ok(vec![DebugConfiguration::Readline { history }]);
        }
    }

    Err(())
}

pub fn parse() -> Result<OptionParser, ()> {
    let config = config_parser().map_err(|e| {
        Logger::show(&e.to_string(), Logger::ERROR);
    })?;

    let cli = CLI::try_parse().map_err(|e| {
        let _ = e.print();
    })?;

    let regions = parse_memory_region(&config, &cli.platform)?;

    let debug_config = parse_debug_configuration(&config).map_err(|_| {
        Logger::show("Invalid debug configuration", Logger::ERROR);
    })?;

    welcome(&cli.platform);

    Ok(OptionParser { memory_config: regions, debug_config, cli })
}
