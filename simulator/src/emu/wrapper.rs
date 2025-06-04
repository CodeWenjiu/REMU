use option_parser::OptionParser;
use remu_utils::ProcessResult;
use state::States;

use crate::{difftest_ref::DifftestRefSingleCycleApi, DirectlyMap, SimulatorCallback, SimulatorItem, SingleCycle};

use super::Emu;

pub trait SimulatorDrive {
    fn step_cycle(emu: &mut Emu) -> ProcessResult<()>;
}

impl SimulatorDrive for DirectlyMap {
    fn step_cycle(emu: &mut Emu) -> ProcessResult<()> {
        emu.self_step_cycle_dm()
    }
}

impl SimulatorDrive for SingleCycle {
    fn step_cycle(emu: &mut Emu) -> ProcessResult<()> {
        emu.self_step_cycle_singlecycle()
    }
}

pub trait DifftestRefSingleCycleDrive {
    fn instruction_compelete(emu: &mut Emu) -> ProcessResult<()>;
}

impl DifftestRefSingleCycleDrive for DirectlyMap {
    fn instruction_compelete(emu: &mut Emu) -> ProcessResult<()> {
        emu.self_step_cycle_dm()
    }
}

impl DifftestRefSingleCycleDrive for SingleCycle {
    fn instruction_compelete(emu: &mut Emu) -> ProcessResult<()> {
        emu.self_step_cycle_singlecycle()
    }
}

pub struct EmuWrapper<V: SimulatorDrive + DifftestRefSingleCycleDrive> {
    emu: Emu,
    _marker: std::marker::PhantomData<V>,
}

impl<V: SimulatorDrive + DifftestRefSingleCycleDrive> SimulatorItem for EmuWrapper<V> {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        V::step_cycle(&mut self.emu)
    }

    fn times(&self) -> ProcessResult<()> {
        self.emu.times()
    }

    fn function_wave_trace(&self,_enable:bool) {
        self.emu.function_wave_trace(_enable)
    }
}

impl<V: SimulatorDrive + DifftestRefSingleCycleDrive> DifftestRefSingleCycleApi for EmuWrapper<V> {
    fn instruction_compelete(&mut self) -> ProcessResult<()> {
        V::instruction_compelete(&mut self.emu)
    }
}

impl<V: SimulatorDrive + DifftestRefSingleCycleDrive> EmuWrapper<V> {
    pub fn new(option: &OptionParser, states: States, callback: SimulatorCallback) -> Self {
        Self {
            emu: Emu::new(option, states, callback),
            _marker: std::marker::PhantomData,
        }
    }
}
