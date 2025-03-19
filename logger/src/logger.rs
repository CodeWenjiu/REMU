use color_eyre::owo_colors::OwoColorize;
use log::{debug, info, trace, warn};
use tracing::error;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Registry};

#[derive(Debug, snafu::Snafu)]
pub enum LoggerError {
    #[snafu(display("Unable to install color_eyre: {}", source))]
    ColorEyreInstall { source: color_eyre::Report },
}

pub enum Logger{
    TRACE,
    DEBUG,
    INFO ,
    WARN ,
    ERROR,
    IMPORTANT,
    SUCCESS,
}

impl From<tracing::Level> for Logger {
    fn from(level: tracing::Level) -> Self {
        match level {
            tracing::Level::TRACE => Logger::TRACE,
            tracing::Level::DEBUG => Logger::DEBUG,
            tracing::Level::INFO => Logger::INFO,
            tracing::Level::WARN => Logger::WARN,
            tracing::Level::ERROR => Logger::ERROR,
        }
    }
}

impl Logger {
    pub fn new() -> Result<(), ()> {
        let file_appender = rolling::never("target/logs", ".log");
        let (non_blocking_appender, _guard) = non_blocking(file_appender);
        let file_layer = fmt::layer()
            .with_ansi(false)
            .with_writer(non_blocking_appender);
            
        Registry::default()
            .with(file_layer)
            .init();

        color_eyre::install().map_err(|e| error!("Unable to install color_eyre: {}", e))?;

        Logger::function("Log", true);

        Ok(())
    }

    pub fn show(message: &str, level: Logger) {
        match level {
            Logger::TRACE       => println!("⏭️  {}", message.white()),
            Logger::DEBUG       => println!("🐞  {}", message.magenta()),
            Logger::INFO        => println!("ℹ️  {}", message.blue()),
            Logger::WARN        => println!("⚠️  {}", message.yellow()),
            Logger::ERROR       => println!("❌  {}", message.green()),
            Logger::IMPORTANT   => println!("✨  {}", message.red()),
            Logger::SUCCESS     => println!("🎉  {}", message.green()),
        }
    }

    pub fn function(function_name: &str, on: bool) {
        let onooff = if on { format!("[{}]", "ON".green()) } else { format!("[{}]", "OFF".red()) };

        println!("🔧  {}{}{}", "function ".blue(), function_name.magenta(), onooff);
    }

    pub fn log(message: &str, level: tracing::Level) {
        match level {
            tracing::Level::TRACE => trace!("{}", message),
            tracing::Level::DEBUG => debug!("{}", message),
            tracing::Level::INFO => info!("{}", message),
            tracing::Level::WARN => warn!("{}", message),
            tracing::Level::ERROR => error!("{}", message),
        }

        Logger::show(message, level.into());
    }
}
