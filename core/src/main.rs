use option_parser::parse;
use simple_debugger::SimpleDebugger;
use tracing::error;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Registry};

fn main() -> Result<(), ()> {
    let cli_result = parse()?;

    let file_appender = rolling::never("logs", ".log");
    let (non_blocking_appender, _guard) = non_blocking(file_appender);
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking_appender);
        
    Registry::default()
        .with(file_layer)
        .init();

    color_eyre::install().map_err(|e| error!("Unable to install color_eyre: {}", e))?;

    let debugger = SimpleDebugger::new(cli_result);
    debugger.mainloop()?;

    Ok(())
}
