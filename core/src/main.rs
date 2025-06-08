use logger::Logger;
use option_parser::{OptionParser, parse};
use simple_debugger::SimpleDebugger;

fn init(option: &OptionParser) -> Result<(), ()> {
    Logger::function("Log", option.cli.log.into());
    if option.cli.log {
        Logger::new()?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    let option = parse()?;

    init(&option)?;

    let debugger = SimpleDebugger::new(option)?;

    debugger.mainloop()?;

    Ok(())
}
