use std::path::PathBuf;
use owo_colors::OwoColorize;

#[derive(Debug, snafu::Snafu)]
pub enum PanicError {
    #[snafu(display("{} {}: {}", "Unable to parse config file from".red(), path.display(), source))]
    ConfigParse { source: config::ConfigError, path: PathBuf },

    #[snafu(display("{}: {}", "Unable to deserilalize file from".red(), source))]
    ConfigDeserialize { source: config::ConfigError }
}

pub type Result<T, E = PanicError> = std::result::Result<T, E>;

remu_macro::mod_flat!(config_parser);
