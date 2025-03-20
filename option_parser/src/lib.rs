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

    // let mut mmu = MMU::new();
    // // mmu.add_memory(0x80000000, 0x80000, "SRAM", MemoryFlags::Read.union(MemoryFlags::Write))
    // //     .map_err(|e| {
    // //         Logger::log(&e.to_string(), tracing::Level::ERROR);
    // //     })?;
    // for each_mem in cli_result
    //     .config
    //     .iter()
    //     .filter(|s| s.0.contains("MEM"))
    //     .collect() {
    //         let mems = each_mem.0.split("_").collect::<Vec<&str>>();
    //     }
    // mmu.show_memory_map();

    welcome(&cli.platform);

    println!("{:?}", config);

    Ok(OptionParser { config, cli })
}
