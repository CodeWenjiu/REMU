// use std::env;
// use nom_kconfig::{parse_kconfig, KconfigInput, KconfigFile};

// // curl https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-6.4.9.tar.xz | tar -xJ -C /tmp/
// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let current_dir = env::current_dir().unwrap();

//     let kconfig_file = KconfigFile::new(
//         current_dir.clone(), 
//         current_dir.join("Kconfig")
//     );
    
//     let input = kconfig_file.read_to_string()?;
//     let input_static: &'static str = Box::leak(input.into_boxed_str());

//     let (_, kconfig) = parse_kconfig(KconfigInput::new_extra(input_static, kconfig_file))?;
    
//     println!("File '{}' contains the following entries:", kconfig.file);
//     kconfig.entries.into_iter().for_each(print_entry);
//     Ok(())
// }

// fn print_entry(entry: nom_kconfig::Entry) {
//     match entry {
//         nom_kconfig::Entry::Config(config) => println!(" - Config '{}'", config.symbol),
//         nom_kconfig::Entry::Menu(menu) => {
//             menu.entries.into_iter().for_each(print_entry);
//         }
//         _ => (),
//     }
// }

use std::collections::HashMap;

use config::Config;

fn main() {
    let settings = Config::builder()
        // Add in `./Settings.toml`
        .add_source(config::File::with_name("config/config"))
        .add_source(config::Environment::with_prefix(""))
        .build()
        .unwrap();

    // Print out our settings (as a HashMap)
    println!(
        "{:?}",
        settings
            .try_deserialize::<HashMap<String, String>>()
            .unwrap()
    );
}