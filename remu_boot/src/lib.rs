use std::sync::Arc;

use remu_debugger::{DebuggerOption, DebuggerRunner};
use remu_harness::{SimulatorNzea, SimulatorRemu};
use remu_state::{StateFastProfile, StateMmioProfile};
use remu_types::{
    DifftestRef, Platform,
    isa::{
        ExtensionSpec, RvIsa,
        extension_enum::{RV32I, RV32I_zve32x_zvl128b, RV32IM, RV32IM_zve32x_zvl128b},
    },
};
use target_lexicon::{Architecture, Riscv32Architecture};

fn boot_with_isa<ISA, Run>(
    option: DebuggerOption,
    runner: Run,
    interrupt: Arc<std::sync::atomic::AtomicBool>,
) where
    ISA: RvIsa,
    Run: DebuggerRunner,
{
    match option.difftest {
        None => runner.run::<SimulatorRemu<StateFastProfile<ISA>, true>, ()>(option, Arc::clone(&interrupt)),
        Some(DifftestRef::Remu) => runner.run::<SimulatorRemu<StateMmioProfile<ISA>, true>, SimulatorRemu<StateMmioProfile<ISA>, false>>(
            option,
            Arc::clone(&interrupt),
        ),
        Some(DifftestRef::Spike) => runner.run::<
            SimulatorRemu<StateMmioProfile<ISA>, true>,
            remu_harness::SimulatorSpike<StateMmioProfile<ISA>>,
        >(option, interrupt),
    }
}

fn boot_with_isa_nzea<ISA, Run>(
    option: DebuggerOption,
    runner: Run,
    interrupt: Arc<std::sync::atomic::AtomicBool>,
) where
    ISA: RvIsa,
    Run: DebuggerRunner,
{
    match option.difftest {
        None => runner.run::<SimulatorNzea<StateFastProfile<ISA>, true>, ()>(option, interrupt),
        Some(DifftestRef::Remu) => runner.run::<
            SimulatorNzea<StateMmioProfile<ISA>, true>,
            SimulatorRemu<StateMmioProfile<ISA>, false>,
        >(option, interrupt),
        Some(DifftestRef::Spike) => {
            panic!("nzea + difftest spike not supported yet")
        }
    }
}

pub fn boot<R: DebuggerRunner>(
    option: DebuggerOption,
    runner: R,
    interrupt: Arc<std::sync::atomic::AtomicBool>,
) {
    use ExtensionSpec::*;
    use Riscv32Architecture::*;

    let isa = &option.isa;
    let platform = option.platform;

    macro_rules! dispatch {
        ($boot_fn:ident) => {
            match (isa.base, isa.extensions) {
                (Architecture::Riscv32(Riscv32i), None) => $boot_fn::<RV32I, R>(option, runner, interrupt),
                (Architecture::Riscv32(Riscv32im), None) => $boot_fn::<RV32IM, R>(option, runner, interrupt),
                (Architecture::Riscv32(Riscv32i), Zve32xZvl128b) => $boot_fn::<RV32I_zve32x_zvl128b, R>(option, runner, interrupt),
                (Architecture::Riscv32(Riscv32im), Zve32xZvl128b) => $boot_fn::<RV32IM_zve32x_zvl128b, R>(option, runner, interrupt),
                (arch, ext) => panic!(
                    "unsupported ISA combination: base={:?}, extensions={:?}",
                    arch, ext
                ),
            }
        };
    }

    if platform == Platform::Nzea {
        dispatch!(boot_with_isa_nzea);
    } else {
        dispatch!(boot_with_isa);
    }
}
