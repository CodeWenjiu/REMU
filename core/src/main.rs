use logger::Logger;
use option_parser::parse;
use simple_debugger::SimpleDebugger;

fn main() -> Result<(), ()> {
    let cli_result = parse()?;

    Logger::new()?;

    let debugger = SimpleDebugger::new(cli_result);
    debugger.mainloop()?;

    Ok(())
}
