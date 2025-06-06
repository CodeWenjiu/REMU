use std::{collections::HashMap, fmt::Debug};

use config::Config;
use logger::Logger;
use regex::Regex;
use remu_macro::{log_debug, log_err, log_error};
use remu_utils::Platform;
use snafu::ResultExt;
use state::mmu::{MMTargetType, MemoryFlags};
use tests::Rule;
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
    pub region_config: Vec<RegionConfiguration>,
    pub debug_config: Vec<DebugConfiguration>,
}

#[derive(Debug)]
pub enum BaseConfiguration {
    ResetVector {
        value: u32,
    },
}

#[derive(Debug)]
pub struct RegionConfiguration {
    pub name: String,
    pub base: u32,
    pub size: u32,
    pub flag: MemoryFlags,
    pub r#type: MMTargetType,
}

#[derive(Debug)]
pub enum DebugConfiguration {
    Readline { history: usize },
    Itrace { enable: bool },
    WaveTrace { enable: bool },
}

pub type ConfigResult<T, E = ConfigError> = std::result::Result<T, E>;

pub fn config_parser() -> ConfigResult<HashMap<String, String>> {
    let path = "config/config";

    let settings = Config::builder()
        .add_source(config::File::with_name(path).format(config::FileFormat::Toml))
        .build()
        .context(ConfigParseSnafu { path })?;
    
    let map = settings.try_deserialize::<HashMap<String, String>>().context(ConfigDeserializeSnafu)?;
    
    Ok(map.into_iter().filter(|(k, _)| k.as_str() != "").collect())
}

fn parse_hex(s: &str) -> Result<u32, ()> {
    let s = s.trim_start_matches("0x");
    log_err!(u32::from_str_radix(s, 16))
}

fn parse_bin(s: &str) -> Result<u32, ()> {
    let s = s.trim_start_matches("0b");
    log_err!(u32::from_str_radix(s, 2))
}

fn parse_dec(s: &str) -> Result<usize, ()> {
    log_err!(usize::from_str_radix(s, 10))
}

fn parse_bool(s: &str) -> Result<bool, ()> {
    match s {
        "y" => Ok(true),
        "n" => Ok(false),
        _ => Err(()),
    }
}

fn parse_base_config_be(
    config: &HashMap<String, String>,
    platform: &Platform,
) -> Result<Vec<BaseConfiguration>, ()> {
    let simulator = Into::<&str>::into(platform.simulator).to_uppercase();

    let mut base_config: Vec<BaseConfiguration> = vec![];

    let re = Regex::new(r"(\w+)_BASE_(\w+)_(\w+)").unwrap();

    for (key, value) in config.iter() {
        if let Some(caps) = re.captures(key) {
            let prefix = &caps[1];

            if prefix.replacen("_", "-", 1) != simulator.to_uppercase() {
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

fn parse_region(
    config: &HashMap<String, String>,
    platform: &Platform,
) -> Result<Vec<RegionConfiguration>, ()> {
    let simulator = Into::<&str>::into(platform.simulator).to_uppercase();

    let mut regions: Vec<RegionConfiguration> = vec![];

    let re = Regex::new(r"(\w+)_MEM_(\w+)_BASE").unwrap();

    for (key, _value) in config.iter() {
        if let Some(caps) = re.captures(key) {
            let prefix = &caps[1];

            if prefix.replacen("_", "-", 1) != simulator.to_uppercase() {
                continue;
            }

            let base_key = format!("{}_MEM_{}_BASE", prefix, &caps[2]);
            let size_key = format!("{}_MEM_{}_SIZE", prefix, &caps[2]);
            let flag_key = format!("{}_MEM_{}_FLAG", prefix, &caps[2]);

            let base_value = config.get(&base_key).map(|v| v);
            let size_value = config.get(&size_key).map(|v| v);
            let flag_value = config.get(&flag_key).map(|v| v);

            if let (Some(base_value), Some(size_value), Some(flag_value)) = (base_value, size_value, flag_value) {
                regions.push(RegionConfiguration {
                    name: caps[2].to_string(),
                    base: parse_hex(base_value)?,
                    size: parse_hex(size_value)?,
                    flag: MemoryFlags::from_bits_truncate(parse_bin(flag_value)? as u8),
                    r#type: MMTargetType::Memory,
                });
            }
        }
    }

    let re = Regex::new(r"^(\w+)_DEV_(\w+)_BASE$").unwrap();

    for (key, _value) in config.iter() {
        if let Some(caps) = re.captures(key) {
            let prefix = &caps[1];

            if prefix.replacen("_", "-", 1) != simulator.to_uppercase() {
                continue;
            }

            let base_key = format!("{}_DEV_{}_BASE", prefix, &caps[2]);
            let size_key = format!("{}_DEV_{}_SIZE", prefix, &caps[2]);
            let flag_key = format!("{}_DEV_{}_FLAG", prefix, &caps[2]);

            let base_value = config.get(&base_key).map(|v| v);
            let size_value = config.get(&size_key).map(|v| v);
            let flag_value = config.get(&flag_key).map(|v| v);

            if let (Some(base_value), Some(size_value), Some(flag_value)) = (base_value, size_value, flag_value) {
                regions.push(RegionConfiguration {
                    name: caps[2].to_string(),
                    base: parse_hex(base_value)?,
                    size: parse_hex(size_value)?,
                    flag: MemoryFlags::from_bits_truncate(parse_bin(flag_value)? as u8),
                    r#type: MMTargetType::Device,
                });
            }
        }
    }

    Ok(regions)
}

fn parse_debug_configuration(
    config: &HashMap<String, String>,
) -> Result<Vec<DebugConfiguration>, ()> {
    let re = Regex::new(r"DEBUG_(\w+)_(\w+)_(\w+)").unwrap();

    let mut debug_config: Vec<DebugConfiguration> = vec![];

    for (key, value) in config.iter() {
        if let Some(caps) = re.captures(key) {
            match (&caps[1], &caps[2], &caps[3]) {
                ("RL", "HISTORY", "SIZE") => {
                    debug_config.push(DebugConfiguration::Readline {
                        history: parse_dec(value)?,
                    });
                }

                ("DEFAULT", "ITRACE", "ENABLE") => {
                    debug_config.push(DebugConfiguration::Itrace {
                        enable: parse_bool(value)?,
                    });
                }

                ("DEFAULT", "WaveTRACE", "ENABLE") => {
                    debug_config.push(DebugConfiguration::WaveTrace {
                        enable: parse_bool(value)?,
                    });
                }

                _ => {
                }
            }
        }
    }

    Ok(debug_config)
}

pub fn config_parse(cli: &CLI) -> Result<Cfg, ()> {
    let config = log_err!(config_parser())?;

    let base_config = parse_base_config_be(&config, &cli.platform)?;

    let regions = parse_region(&config, &cli.platform)?;

    let debug_config = parse_debug_configuration(&config).map_err(|_| {
        Logger::show("Invalid debug configuration", Logger::ERROR);
    })?;

    Ok(Cfg {
        base_config,
        region_config: regions,
        debug_config,
    })
}

fn parse_base_config(
    pairs: pest::iterators::Pairs<'_, Rule>,
) -> Result<Vec<DebugConfiguration>, ()> {
    let mut result = vec![];

    for pair in pairs {
        match pair.as_rule() {
            Rule::rl_history_size => {
                result.push(
                    DebugConfiguration::Readline {
                        history: parse_dec(pair.as_str())?,
                    }
                );
            }
            _ => unreachable!()
        }
    }

    Ok(result)
}

fn parse_statement(
    pairs: pest::iterators::Pairs<'_, Rule>,
    // platform: &Platform,
) -> Result<Vec<DebugConfiguration>, ()> {
    let mut result = vec![];

    for pair in pairs {
        match pair.as_rule() {
            Rule::config_debug => {
                result = parse_base_config(pair.into_inner())?;
            }

            Rule::config_platform => {}

            Rule::config_ignore => {}

            _ => unreachable!()
        }
    }

    Ok(result)
}

fn parse_file(
    pairs: pest::iterators::Pairs<'_, Rule>,
    // platform: &Platform,
) -> Result<Vec<DebugConfiguration>, ()> {
    let mut result = vec![];

    for pair in pairs {
        match pair.as_rule() {
            Rule::config_statement => {
                result = parse_statement(pair.into_inner())?
            },
            Rule::EOI => {}
            _ => unreachable!()
        }
    }       

    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::fs::read;
    use pest::Parser;
    use pest_derive::Parser;

    use crate::config_parser::parse_file;
    
    #[derive(Parser)]
    #[grammar = "config_parser.pest"]
    struct ConfigParser;

    #[test]
    fn pest_test() {
        let src = read("../config/.config").unwrap();
        let src = String::from_utf8(src).unwrap();
        let pairs = ConfigParser::parse(Rule::file, &src).unwrap();

        let result = parse_file(pairs).unwrap();
        println!("{:?}", result);
    }
}