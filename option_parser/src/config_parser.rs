use std::{collections::HashMap, fmt::Debug};

use config::Config;
use logger::Logger;
use regex::Regex;
use remu_utils::Platform;
use snafu::ResultExt;
use state::mmu::MemoryFlags;
use std::path::PathBuf;
use owo_colors::OwoColorize;

use crate::CLI;

#[derive(Debug, snafu::Snafu)]
pub enum ConfigError {
    #[snafu(display("{} {}: {}", "Unable to parse config file from".red(), path.display(), source))]
    ConfigParse { source: config::ConfigError, path: PathBuf },

    #[snafu(display("{}: {}", "Unable to deserilalize file from".red(), source))]
    ConfigDeserialize { source: config::ConfigError }
}

pub struct Cfg {
    pub base_config: Vec<BaseConfiguration>,
    pub memory_config: Vec<MemoryConfiguration>,
    pub debug_config: Vec<DebugConfiguration>,
}

#[derive(Debug)]
pub enum BaseConfiguration {
    ResetVector {
        value: u32,
    },
}

#[derive(Debug)]
pub enum MemoryConfiguration {
    MemoryRegion {
        name: String,
        base: u32,
        size: u32,
        flag: MemoryFlags,
    },
}

#[derive(Debug)]
pub enum DebugConfiguration {
    Readline { history: u32 },
}

pub type ConfigResult<T, E = ConfigError> = std::result::Result<T, E>;

pub fn config_parser() -> ConfigResult<HashMap<String, String>> {
    let path = "config/config";

    let settings = Config::builder()
        .add_source(config::File::with_name(path).format(config::FileFormat::Toml))
        .add_source(config::Environment::with_prefix("").separator("_"))
        .build()
        .context(ConfigParseSnafu { path })?;
    
    let map = settings.try_deserialize::<HashMap<String, String>>().context(ConfigDeserializeSnafu)?;
    
    Ok(map.into_iter().filter(|(k, _)| k.as_str() != "").collect())
}

fn parse_hex(s: &str) -> Result<u32, ()> {
    let s = s.trim_start_matches("0x");
    u32::from_str_radix(s, 16).map_err(|e| Logger::show(&e.to_string(), Logger::ERROR))
}

fn parse_bin(s: &str) -> Result<u32, ()> {
    let s = s.trim_start_matches("0b");
    u32::from_str_radix(s, 2).map_err(|e| Logger::show(&e.to_string(), Logger::ERROR))
}

fn parse_dec(s: &str) -> Result<u32, ()> {
    u32::from_str_radix(s, 10).map_err(|e| Logger::show(&e.to_string(), Logger::ERROR))
}

fn parse_base_config(
    config: &HashMap<String, String>,
    platform: &Platform,
) -> Result<Vec<BaseConfiguration>, ()> {
    let simulator = Into::<&str>::into(platform.simulator).to_uppercase();

    let mut base_config: Vec<BaseConfiguration> = vec![];

    let re = Regex::new(r"(\w+)_BASE_(\w+)_(\w+)").unwrap();

    for (key, value) in config.iter() {
        if let Some(caps) = re.captures(key) {
            let prefix = &caps[1];

            if prefix != &simulator.to_uppercase() {
                continue;
            }

            let base_key = format!("{}_BASE_{}_{}", prefix, &caps[2], &caps[3]);

            if let Some(_) = config.get(&base_key) {
                match &caps[2] {
                    "RESET" => {
                        base_config.push(BaseConfiguration::ResetVector {
                            value: parse_hex(value)?,
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(base_config)
}

fn parse_memory_region(
    config: &HashMap<String, String>,
    platform: &Platform,
) -> Result<Vec<MemoryConfiguration>, ()> {
    let simulator = Into::<&str>::into(platform.simulator).to_uppercase();

    let mut regions: Vec<MemoryConfiguration> = vec![];

    let re = Regex::new(r"(\w+)_MEM_(\w+)_BASE").unwrap();

    for (key, _value) in config.iter() {
        if let Some(caps) = re.captures(key) {
            let prefix = &caps[1];

            if prefix != &simulator.to_uppercase() {
                continue;
            }

            let base_key = format!("{}_MEM_{}_BASE", prefix, &caps[2]);
            let size_key = format!("{}_MEM_{}_SIZE", prefix, &caps[2]);
            let flag_key = format!("{}_MEM_{}_FLAG", prefix, &caps[2]);

            let base_value = config.get(&base_key).map(|v| v);
            let size_value = config.get(&size_key).map(|v| v);
            let flag_value = config.get(&flag_key).map(|v| v);

            if let (Some(base_value), Some(size_value), Some(flag_value)) = (base_value, size_value, flag_value) {
                regions.push(MemoryConfiguration::MemoryRegion {
                    name: caps[2].to_string(),
                    base: parse_hex(base_value)?,
                    size: parse_hex(size_value)?,
                    flag: MemoryFlags::from_bits_truncate(parse_bin(flag_value)? as u8),
                });
            }
        }
    }

    Ok(regions)
}

fn parse_debug_configuration(
    config: &HashMap<String, String>,
) -> Result<Vec<DebugConfiguration>, ()> {
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

pub fn config_parse(cli: &CLI) -> Result<Cfg, ()> {
    let config = config_parser().map_err(|e| {
        Logger::show(&e.to_string(), Logger::ERROR);
    })?;

    let base_config = parse_base_config(&config, &cli.platform)?;

    let regions = parse_memory_region(&config, &cli.platform)?;

    let debug_config = parse_debug_configuration(&config).map_err(|_| {
        Logger::show("Invalid debug configuration", Logger::ERROR);
    })?;

    Ok(Cfg {
        base_config,
        memory_config: regions,
        debug_config,
    })
}
