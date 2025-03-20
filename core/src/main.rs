use logger::Logger;
use option_parser::parse;
use simple_debugger::SimpleDebugger;

fn main() -> Result<(), ()> {
    let cli_result = parse()?;
    
    if cli_result.cli.log {
        Logger::new()?;
    }
    Logger::function("Log", cli_result.cli.log);

    let debugger = SimpleDebugger::new(cli_result);
    debugger.mainloop()?;

    Ok(())
}
