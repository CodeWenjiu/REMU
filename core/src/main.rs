use logger::Logger;
use option_parser::parse;
use simple_debugger::SimpleDebugger;
use state::mmu::{MemoryFlags, MMU};

fn main() -> Result<(), ()> {
    let cli_result = parse()?;
    
    if cli_result.cli.log {
        Logger::new()?;
    }
    Logger::function("Log", cli_result.cli.log);

    let mut mmu = MMU::new();
    mmu.add_memory(0x80000000, 0x80000, "SRAM", MemoryFlags::Read.union(MemoryFlags::Write))
        .map_err(|e| {
            Logger::log(&e.to_string(), tracing::Level::ERROR);
        })?;
    mmu.add_memory(0x80008000, 0x8000, "TEST", MemoryFlags::Read.union(MemoryFlags::Write))
        .map_err(|e| {
            Logger::log(&e.to_string(), tracing::Level::ERROR);
        })?;
    mmu.show_memory_map();

    let debugger = SimpleDebugger::new(cli_result);
    debugger.mainloop()?;

    Ok(())
}
