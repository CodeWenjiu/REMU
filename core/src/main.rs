use std::{collections::HashMap, path::PathBuf};

use config::Config;
use owo_colors::OwoColorize;
use snafu::ResultExt;

#[derive(Debug, snafu::Snafu)]
enum PanicError {
    #[snafu(display("{} {}: {}", "Unable to parse config file from".red(), path.display(), source))]
    ConfigParse { source: config::ConfigError, path: PathBuf },

    #[snafu(display("{}: {}", "Unable to deserilalize file from".red(), source))]
    ConfigDeserialize { source: config::ConfigError }
}

type Result<T, E = PanicError> = std::result::Result<T, E>;

#[snafu::report]
fn main() -> Result<()> {
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