use std::collections::HashMap;

use config::Config;

fn main() {
    let settings = Config::builder()
        // Add in `./Settings.toml`
        .add_source(config::File::with_name("config/config"))
        .add_source(config::Environment::with_prefix(""))
        .build()
        .unwrap();

    // println!(
    //     "{:?}",
    //     settings
    //         .try_deserialize::<HashMap<String, String>>()
    //         .unwrap()
    // );
    // print iterator

    let map = settings.try_deserialize::<HashMap<String, String>>().unwrap();
    for (key, value) in map.iter().filter(|(k, _)| k.as_str() != "") {
        println!("{}: {}", key, value);
    }
}