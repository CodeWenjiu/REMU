use tracing::error;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Registry};

#[derive(Debug, snafu::Snafu)]
pub enum LoggerError {
    #[snafu(display("Unable to install color_eyre: {}", source))]
    ColorEyreInstall { source: color_eyre::Report },
}

pub struct Logger;

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

        Ok(())
    }
}
