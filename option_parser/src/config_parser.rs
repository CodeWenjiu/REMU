use std::{fmt::Debug, fs::read};

use logger::Logger;
use pest::Parser;
use remu_macro::log_err;
use remu_utils::Platform;
use state::{cache::CacheConfiguration, mmu::{MMTargetType, MemoryFlags, RegionConfiguration}};
use std::path::PathBuf;

#[derive(Debug)]
pub struct AllCacheConfiguration {
    pub btb: Option<CacheConfiguration>,

    pub icache: Option<CacheConfiguration>,

    pub dcache: Option<CacheConfiguration>,
}

#[derive(Debug, Default)]
pub struct DebugConfiguration {
    pub rl_history_size: usize,
    pub itrace_enable: bool,
    pub wave_trace_enable: bool,
}

fn parse_bool(s: &str) -> Result<bool, ()> {
    match s {
        "y" => Ok(true),
        "n" => Ok(false),
        _ => Err(()),
    }
}

#[derive(Debug)]
pub struct PlatformConfiguration {
    pub reset_vector: u32,
    pub regions: Vec<RegionConfiguration>,
    pub cache: AllCacheConfiguration,
}

impl Default for PlatformConfiguration {
    fn default() -> Self {
        Self {
            reset_vector: 0x8000_0000,
            regions: vec![RegionConfiguration {
                name: String::from("default"),
                base: 0x8000_0000,
                size: 0x0800_0000,
                flag: MemoryFlags::all(),
                mmtype: MMTargetType::Memory,
            }],
            cache: AllCacheConfiguration {
                btb: None,
                icache: None,
                dcache: None,
            },
        }
    }
}

#[derive(Debug, Default)]
pub struct Cfg {
    pub debug_config: DebugConfiguration,
    pub platform_config: PlatformConfiguration,
}
    
use pest_derive::Parser;
#[derive(Parser)]
#[grammar = "config_parser.pest"]
struct ConfigParser;

fn parse_debug_config(pairs: pest::iterators::Pairs<'_, Rule>, result: &mut DebugConfiguration) -> Result<(), ()> {
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

fn parse_cache_config(pairs: pest::iterators::Pairs<'_, Rule>) -> Result<(String, CacheConfiguration), ()> {
    let mut name = String::new();
    let mut set = 0;
    let mut way = 0;
    let mut block_num = 0;
    let mut replacement = String::new();

    for pair in pairs {
        match pair.as_rule() {
            Rule::target_cache_name => {
                name = pair.as_str().to_string();
            }

            Rule::target_cache_set => {
                set = log_err!(u32::from_str_radix(pair.as_str(), 10))?;
            }

            Rule::target_cache_way => {
                way = log_err!(u32::from_str_radix(pair.as_str(), 10))?;
            }

            Rule::target_cache_blocknum => {
                block_num = log_err!(u32::from_str_radix(pair.as_str(), 10))?;
            }

            Rule::target_cache_replacement => {
                replacement = pair.as_str().to_string();
            }

            _ => unreachable!()
        }
    }

    Ok((name, CacheConfiguration { set, way, block_num, replacement }))
}

fn parse_platform_config(pairs: pest::iterators::Pairs<'_, Rule>, result: &mut PlatformConfiguration, platform: &Platform) -> Result<(), ()> {
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
            
            Rule::target_cache => {
                let parse = parse_cache_config(pair.into_inner())?;
                match parse.0.as_str() {
                    "BTB" => {
                        result.cache.btb = Some(parse.1);
                    }

                    "ICache" => {
                        result.cache.icache = Some(parse.1);
                    }

                    "DCache" => {
                        result.cache.dcache = Some(parse.1);
                    }

                    _ => unreachable!(),
                }
            }

            _ => unreachable!()
        }
    }
    Ok(())
}

fn parse_config_statement(pair: pest::iterators::Pair<'_, Rule>, result: &mut Cfg, platform: &Platform) -> Result<(), ()> {
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

pub fn parse_config(config_path: PathBuf, platform: &Platform) -> Result<Cfg, ()> {
    Logger::show(
        &format!("Parsing config file: {}", config_path.display()).to_string(),
        Logger::INFO,
    );
    
    let src = read(config_path).unwrap();
    let src = String::from_utf8(src).unwrap();
    let pairs = ConfigParser::parse(Rule::file, &src).unwrap();

    let mut result = Cfg{
        debug_config: DebugConfiguration::default(),
        platform_config: PlatformConfiguration{
            reset_vector: 0x8000_0000,
            regions: vec![],
            cache: AllCacheConfiguration { btb: None, icache: None, dcache: None},
        },
    };
    
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