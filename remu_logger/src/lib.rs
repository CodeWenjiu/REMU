use std::path::Path;

use anyhow::Result;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{Registry, layer::SubscriberExt, util::SubscriberInitExt};

#[tracing::instrument]
pub fn set_logger(
    log_dir: impl AsRef<Path> + std::fmt::Debug,
    file_name: &str,
) -> Result<(WorkerGuard, WorkerGuard)> {
    let file_appender = tracing_appender::rolling::never(log_dir, file_name);
    let (stdout, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());
    let (file, file_guard) = tracing_appender::non_blocking(file_appender);

    let console_layer = tracing_subscriber::fmt::layer()
        // Output to stdout
        .with_writer(stdout)
        // Use a more pretty, human-readable log format
        .pretty()
        // Use ANSI colors for output
        .with_ansi(true)
        // Dont display the timestamp
        .without_time()
        // Display source code file paths
        .with_file(true)
        // Display source code line numbers
        .with_line_number(true)
        // Display the thread ID an event was recorded on
        .with_thread_ids(true)
        // Don't display the event's target (module path)
        .with_target(false);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file)
        .with_ansi(false)
        .with_file(true)
        .with_line_number(true)
        .with_target(true);

    Registry::default()
        .with(console_layer)
        .with(file_layer)
        .try_init()?;

    Ok((stdout_guard, file_guard))
}

#[macro_export]
macro_rules! init_logger {
    ($dir:expr, $file:expr) => {
        let (_stdout_guard, _file_guard) = $crate::set_logger($dir, $file)?;
    };
}
