use std::{fmt::Debug, fs::read};

use logger::Logger;
use pest::Parser;
use remu_macro::log_err;
use remu_utils::Platform;
use state::mmu::{MMTargetType, MemoryFlags};
use std::path::PathBuf;

#[derive(Debug)]
pub struct RegionConfiguration {
    pub name: String,
    pub base: u32,
    pub size: u32,
    pub flag: MemoryFlags,
    pub mmtype: MMTargetType,
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

#[derive(Debug, Default)]
pub struct PlatformConfiguration {
    pub reset_vector: u32,
    pub regions: Vec<RegionConfiguration>,
}

#[derive(Debug, Default)]
pub struct CfgAf {
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