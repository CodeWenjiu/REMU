use std::collections::HashMap;

use config::Config;
use snafu::ResultExt;
use std::path::PathBuf;
use owo_colors::OwoColorize;

#[derive(Debug, snafu::Snafu)]
pub enum ConfigError {
    #[snafu(display("{} {}: {}", "Unable to parse config file from".red(), path.display(), source))]
    ConfigParse { source: config::ConfigError, path: PathBuf },

    #[snafu(display("{}: {}", "Unable to deserilalize file from".red(), source))]
    ConfigDeserialize { source: config::ConfigError }
}

pub type ConfigResult<T, E = ConfigError> = std::result::Result<T, E>;

pub fn config_parser() -> ConfigResult<HashMap<String, String>> {
    let path = "config/config";

    let settings = Config::builder()
        .add_source(config::File::with_name(path))
        .add_source(config::Environment::with_prefix(""))
        .build()
        .context(ConfigParseSnafu { path })?;
    
    let map = settings.try_deserialize::<HashMap<String, String>>().context(ConfigDeserializeSnafu)?;
    
    Ok(map.into_iter().filter(|(k, _)| k.as_str() != "").collect())
}
