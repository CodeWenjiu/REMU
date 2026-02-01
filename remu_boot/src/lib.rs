use remu_debugger::{DebuggerOption, DebuggerRunner};
use target_lexicon::{Architecture, Riscv32Architecture};

use crate::isa_dispatch::{boot_riscv32i, boot_riscv32im};

mod isa_dispatch {
    use remu_debugger::{DebuggerProfile, DebuggerRunner};
    use remu_simulator::{SimulatorFastProfile, SimulatorMmioProfile};
    use remu_types::{isa::extension_enum::RV32I, isa::extension_enum::RV32IM, isa::RvIsa, DifftestRef};

    /// 按 difftest 选择 SimulatorProfile 并派发：无 difftest 用 Fast，有则用 Mmio（与 ref 同步）。
    fn boot_with_isa<ISA, R>(option: remu_debugger::DebuggerOption, runner: R, difftest: Option<DifftestRef>)
    where
        ISA: RvIsa,
        R: DebuggerRunner,
    {
        match difftest {
            None => runner.run::<DebuggerProfile<ISA, SimulatorFastProfile<ISA>>>(option),
            Some(DifftestRef::Remu) => runner.run::<DebuggerProfile<ISA, SimulatorMmioProfile<ISA>>>(option),
        }
    }

    pub fn boot_riscv32i<R: DebuggerRunner>(option: remu_debugger::DebuggerOption, runner: R) {
        let difftest = option.difftest;
        boot_with_isa::<RV32I, R>(option, runner, difftest);
    }

    pub fn boot_riscv32im<R: DebuggerRunner>(option: remu_debugger::DebuggerOption, runner: R) {
        let difftest = option.difftest;
        boot_with_isa::<RV32IM, R>(option, runner, difftest);
    }
}

pub fn boot<R: DebuggerRunner>(option: DebuggerOption, runner: R) {
    match option.isa.0 {
        Architecture::Riscv32(arch) => match arch {
            Riscv32Architecture::Riscv32i => boot_riscv32i(option, runner),
            Riscv32Architecture::Riscv32im => boot_riscv32im(option, runner),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}
