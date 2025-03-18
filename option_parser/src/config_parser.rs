use std::collections::HashMap;

use config::Config;
use snafu::ResultExt;

use crate::{ConfigDeserializeSnafu, ConfigParseSnafu};

type Result = crate::Result<()>;

pub fn config_parser() -> Result {
    let path = "config/config";

    let settings = Config::builder()
        // Add in `./Settings.toml`
        .add_source(config::File::with_name(path))
        .add_source(config::Environment::with_prefix(""))
        .build()
        .context(ConfigParseSnafu { path })?;
    
    let map = settings.try_deserialize::<HashMap<String, String>>().context(ConfigDeserializeSnafu)?;
    for (key, value) in map.iter().filter(|(k, _)| k.as_str() != "") {
        println!("{}: {}", key, value);
    }
    
    Ok(())
}