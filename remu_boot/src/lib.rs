use remu_debugger::{DebuggerOption, DebuggerRunner};
use remu_harness::SimulatorRemu;
use remu_state::{StateFastProfile, StateMmioProfile};
use remu_types::{
    DifftestRef,
    isa::RvIsa,
    isa::extension_enum::{RV32I, RV32IM},
};
use target_lexicon::{Architecture, Riscv32Architecture};

fn boot_with_isa<ISA, Run>(option: DebuggerOption, runner: Run)
where
    ISA: RvIsa,
    Run: DebuggerRunner,
{
    let difftest = option.difftest;
    match difftest {
        None => runner.run::<StateFastProfile<ISA>, ()>(option),
        Some(DifftestRef::Remu) => {
            runner.run::<StateMmioProfile<ISA>, SimulatorRemu<StateMmioProfile<ISA>>>(option)
        }
    }
}

pub fn boot<R: DebuggerRunner>(option: DebuggerOption, runner: R) {
    match option.isa.0 {
        Architecture::Riscv32(arch) => match arch {
            Riscv32Architecture::Riscv32i => boot_with_isa::<RV32I, R>(option, runner),
            Riscv32Architecture::Riscv32im => boot_with_isa::<RV32IM, R>(option, runner),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}
