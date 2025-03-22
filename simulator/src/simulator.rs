use logger::Logger;
use remu_utils::{Platform, ProcessResult, Simulators, ISA};
use enum_dispatch::enum_dispatch;

use crate::emu::Emu;

#[enum_dispatch]
pub trait Simulator {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        Logger::todo();
        Ok(())
    }
}

#[enum_dispatch(Simulator)]
pub enum SimulatorImpl {
    NEMU(Emu),
}

impl From<(Simulators, ISA)> for SimulatorImpl {
    fn from(sim_isa: (Simulators, ISA)) -> Self {
        let (sim, isa) = sim_isa;
        match sim {
            Simulators::EMU => SimulatorImpl::NEMU(Emu::new(isa)),
            Simulators::NPC => {
                Logger::todo();
                SimulatorImpl::NEMU(Emu::new(isa))
            }
        }
    }
}

impl From<&Platform> for SimulatorImpl {
    fn from(platform: &Platform) -> Self {
        (platform.simulator, platform.isa).into()
    }
}
