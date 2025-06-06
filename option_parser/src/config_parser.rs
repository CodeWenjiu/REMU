use std::{collections::HashMap, fmt::Debug, fs::read};

use config::Config;
use logger::Logger;
use pest::Parser;
use regex::Regex;
use remu_macro::log_err;
use remu_utils::Platform;
use snafu::ResultExt;
use state::mmu::{MMTargetType, MemoryFlags};
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
    pub mmtype: MMTargetType,
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
                    mmtype: MMTargetType::Memory,
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
                    mmtype: MMTargetType::Device,
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

#[derive(Debug, Default)]
pub struct DebugConfigurationAf {
    pub rl_history_size: usize,
    pub itrace_enable: bool,
    pub wave_trace_enable: bool,
}

#[derive(Debug, Default)]
pub struct PlatformConfigurationAf {
    pub reset_vector: u32,
    pub regions: Vec<RegionConfiguration>,
}

#[derive(Debug, Default)]
pub struct CfgAf {
    pub debug_config: DebugConfigurationAf,
    pub platform_config: PlatformConfigurationAf,
}
    
use pest_derive::Parser;
#[derive(Parser)]
#[grammar = "config_parser.pest"]
struct ConfigParser;

fn parse_debug_config(pairs: pest::iterators::Pairs<'_, Rule>, result: &mut DebugConfigurationAf) -> Result<(), ()> {
    for pair in pairs {
        match pair.as_rule() {
            Rule::rl_history_size => {
                result.rl_history_size = log_err!(usize::from_str_radix(pair.as_str(), 10))?;
            }

            Rule::itrace_enable => {
                result.itrace_enable = parse_bool(pair.as_str())?;
            }

            Rule::wave_trace_enable => {
                result.wave_trace_enable = parse_bool(pair.as_str())?;
            }

            _ => unreachable!()
        }
    }
    Ok(())
}

fn parse_base_config(pairs: pest::iterators::Pairs<'_, Rule>) -> Result<u32, ()> {
    let mut result = 0;

    for pair in pairs {
        match pair.as_rule() {
            Rule::reset_vector_value => {
                result = log_err!(u32::from_str_radix(pair.as_str(), 16))?;
            }

            _ => unreachable!()
        }
    }

    Ok(result)
}

fn parse_region_config(pairs: pest::iterators::Pairs<'_, Rule>) -> Result<RegionConfiguration, ()> {
    let mut result = RegionConfiguration {
        name: String::new(),
        base: 0,
        size: 0,
        flag: MemoryFlags::empty(),
        mmtype: MMTargetType::Memory,
    };

    for pair in pairs {
        match pair.as_rule() {
            Rule::target_mem_region => {
                result.name = pair.as_str().to_string();
            }

            Rule::target_dev_region => {
                result.name = pair.as_str().to_string();
                result.mmtype = MMTargetType::Device;
            }

            Rule::region_base => {
                result.base = log_err!(u32::from_str_radix(pair.as_str(), 16))?;
            }

            Rule::region_size => {
                result.size = log_err!(u32::from_str_radix(pair.as_str(), 16))?;
            }

            Rule::region_flag => {
                result.flag = MemoryFlags::from_bits_truncate(log_err!(u8::from_str_radix(pair.as_str(), 2))?);
            }

            _ => unreachable!("{}", pair.as_str())
        }
    }

    Ok(result)
}

fn parse_platform_config(pairs: pest::iterators::Pairs<'_, Rule>, result: &mut PlatformConfigurationAf, platform: &Platform) -> Result<(), ()> {
    for pair in pairs {
        match pair.as_rule() {
            Rule::platform => {
                let (platform, _target) = Into::<&str>::into(platform.simulator).split_once('-').unwrap();
                if pair.as_str() != platform {
                    return Ok(());
                }
            }

            Rule::target => {
                let (_, target) = Into::<&str>::into(platform.simulator).split_once('-').unwrap();
                if pair.as_str() != target {
                    return Ok(());
                }
            }
            
            Rule::target_base => result.reset_vector = parse_base_config(pair.into_inner())?,

            Rule::target_region => result.regions.push(parse_region_config(pair.into_inner())?),
            
            _ => unreachable!()
        }
    }
    Ok(())
}

fn parse_config_statement(pair: pest::iterators::Pair<'_, Rule>, result: &mut CfgAf, platform: &Platform) -> Result<(), ()> {
    for pair in pair.into_inner() {
        match pair.as_rule() {
            Rule::config_debug => parse_debug_config(pair.into_inner(), &mut result.debug_config)?,

            Rule::config_platform => parse_platform_config(pair.into_inner(), &mut result.platform_config, platform)?,
            
            Rule::config_ignore => {}

            _ => unreachable!()
        }
    }
    Ok(())
}

pub fn parse_config(config_path: PathBuf, platform: &Platform) -> Result<CfgAf, ()> {
    // let src = read("../config/.config").unwrap();
    let src = read(config_path).unwrap();
    let src = String::from_utf8(src).unwrap();
    let pairs = ConfigParser::parse(Rule::file, &src).unwrap();

    let mut result = CfgAf::default();
    
    for pair in pairs {
        match pair.as_rule() {
            Rule::config_statement => {
                parse_config_statement(pair, &mut result, platform)?;
            }
            Rule::EOI => {}
            _ => unreachable!()
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr};

    use owo_colors::OwoColorize;
    use remu_utils::Platform;

    use crate::{config_parser::{parse_config}};

    #[test]
    fn pest_test() {

        let platform = Platform::from_str("rv32i-emu-sc").unwrap();

        let result = parse_config("../config/.config".into(), &platform).unwrap();
        println!("{:#?}", result.green());
    }
}