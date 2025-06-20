use logger::{FeatureState, Logger};
use option_parser::parse;
use simple_debugger::SimpleDebugger;

fn init() -> Result<(), ()> {
    cfg_if::cfg_if! {
        if #[cfg(feature = "LOG")] {
            Logger::new()?;
            Logger::function("Log", FeatureState::Active);
        } else {
            Logger::function("Log", FeatureState::Disabled);
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    let option = parse()?;

    init()?;

    let exec = option.cli.exec.clone();

    let debugger = SimpleDebugger::new(option)?;

    debugger.mainloop(exec)?;

    Ok(())
}
