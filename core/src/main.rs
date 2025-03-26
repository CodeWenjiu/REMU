use logger::Logger;
use option_parser::{parse, OptionParser};
use simple_debugger::SimpleDebugger;

fn init(option: &OptionParser) -> Result<(), ()> {
    Logger::function("Log", option.cli.log);
    if option.cli.log {
        Logger::new()?;
    }
    Ok(())
}

fn main() -> Result<(), ()> {
    let option = parse()?;

    init(&option)?;

    let debugger = SimpleDebugger::new(option)?;

    debugger.mainloop()?;

    Ok(())
}
