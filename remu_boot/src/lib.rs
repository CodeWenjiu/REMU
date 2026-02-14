use std::sync::Arc;

use remu_debugger::{DebuggerOption, DebuggerRunner};
use remu_harness::RefSim;
use remu_state::{StateFastProfile, StateMmioProfile};
use remu_types::{
    DifftestRef,
    isa::{
        extension_enum::{RV32I, RV32I_zve32x_zvl128b, RV32IM},
        ExtensionSpec, RvIsa,
    },
};
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
        Some(DifftestRef::Spike) => {
            runner.run::<StateMmioProfile<ISA>, remu_simulator_spike::SimulatorSpike<StateMmioProfile<ISA>>>(
                option, interrupt,
            );
        }
    }
}

pub fn boot<R: DebuggerRunner>(option: DebuggerOption, runner: R, interrupt: Arc<std::sync::atomic::AtomicBool>) {
    use ExtensionSpec::*;
    use Riscv32Architecture::*;

    let isa = &option.isa;
    match (isa.base, isa.extensions) {
        (Architecture::Riscv32(Riscv32i), None) => {
            boot_with_isa::<RV32I, R>(option, runner, interrupt)
        }
        (Architecture::Riscv32(Riscv32im), None) => {
            boot_with_isa::<RV32IM, R>(option, runner, interrupt)
        }
        (Architecture::Riscv32(Riscv32i), Zve32xZvl128b) => {
            boot_with_isa::<RV32I_zve32x_zvl128b, R>(option, runner, interrupt)
        }
        (arch, ext) => {
            panic!("unsupported ISA combination: base={:?}, extensions={:?}", arch, ext);
        }
    }
}
