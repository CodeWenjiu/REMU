use std::marker::PhantomData;

use remu_simulator::{SimulatorFastProfile, SimulatorPolicy};
use remu_types::isa::{
    RvIsa,
    extension_enum::{RV32I, RV32IM},
};
use target_lexicon::{Architecture, Riscv32Architecture};

use crate::DebuggerOption;

pub trait DebuggerPolicy {
    type SimPolicy: SimulatorPolicy;
}

pub struct DebuggerProfile<ISA, SimProlicy>
where
    ISA: RvIsa,
    SimProlicy: SimulatorPolicy,
{
    _marker_isa: PhantomData<(ISA, SimProlicy)>,
}

impl<ISA, SimProlicy> DebuggerPolicy for DebuggerProfile<ISA, SimProlicy>
where
    ISA: RvIsa,
    SimProlicy: SimulatorPolicy,
{
    type SimPolicy = SimProlicy;
}

pub trait DebuggerRunner {
    fn run<P: DebuggerPolicy>(self, option: DebuggerOption);
}

pub struct DebuggerBootLoader;

impl DebuggerBootLoader {
    pub fn boot(option: DebuggerOption, runner: impl DebuggerRunner) {
        match option.isa.0 {
            Architecture::Riscv32(arch) => match arch {
                Riscv32Architecture::Riscv32i => {
                    runner.run::<DebuggerProfile<RV32I, SimulatorFastProfile<RV32I>>>(option)
                }

                Riscv32Architecture::Riscv32im => {
                    runner.run::<DebuggerProfile<RV32I, SimulatorFastProfile<RV32IM>>>(option)
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
}
