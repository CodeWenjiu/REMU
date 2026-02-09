use std::sync::Arc;

use remu_debugger::{DebuggerOption, DebuggerRunner};
use remu_harness::RefSim;
use remu_state::{StateFastProfile, StateMmioProfile};
use remu_types::{DifftestRef, isa::RvIsa, isa::extension_enum::{RV32I, RV32IM}};
use target_lexicon::{Architecture, Riscv32Architecture};

fn boot_with_isa<ISA, Run>(option: DebuggerOption, runner: Run, interrupt: Arc<std::sync::atomic::AtomicBool>)
where
    ISA: RvIsa,
    Run: DebuggerRunner,
{
    match option.difftest {
        None => runner.run::<StateFastProfile<ISA>, ()>(option, Arc::clone(&interrupt)),
        Some(DifftestRef::Remu) => {
            runner.run::<StateMmioProfile<ISA>, RefSim<StateMmioProfile<ISA>>>(option, Arc::clone(&interrupt))
        }
        Some(DifftestRef::Unicorn) => {
            runner.run::<StateMmioProfile<ISA>, remu_simulator_unicorn::SimulatorUnicorn<StateMmioProfile<ISA>>>(option, interrupt);
        }
    }
}

pub fn boot<R: DebuggerRunner>(option: DebuggerOption, runner: R, interrupt: Arc<std::sync::atomic::AtomicBool>) {
    match option.isa.0 {
        Architecture::Riscv32(Riscv32Architecture::Riscv32i) => boot_with_isa::<RV32I, R>(option, runner, interrupt),
        Architecture::Riscv32(Riscv32Architecture::Riscv32im) => {
            boot_with_isa::<RV32IM, R>(option, runner, interrupt)
        }
        _ => unreachable!(),
    }
}
