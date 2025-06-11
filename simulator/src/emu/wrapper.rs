use option_parser::OptionParser;
use remu_utils::ProcessResult;
use state::States;

use crate::{difftest_ref::{DifftestRefPipelineApi, DifftestRefSingleCycleApi}, DirectlyMap, Pipeline, SimulatorCallback, SimulatorItem, SingleCycle};

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

impl SimulatorDrive for Pipeline {
    fn step_cycle(emu: &mut Emu) -> ProcessResult<()> {
        emu.self_step_cycle_pipeline()
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

pub trait DifftestRefPipelineDrive {
    fn step_cycle(emu: &mut Emu) -> ProcessResult<()>;

    fn instruction_fetch_enable(emu: &mut Emu);
    fn load_store_enable(emu: &mut Emu);
}

impl DifftestRefPipelineDrive for Pipeline {
    fn step_cycle(emu: &mut Emu) -> ProcessResult<()> {
        emu.self_step_cycle_pipeline_without_enable()
    }

    fn instruction_fetch_enable(emu: &mut Emu) {
        emu.self_pipeline_ifena();
    }

    fn load_store_enable(emu: &mut Emu) {
        emu.self_pipeline_lsena();
    }
}

pub trait SimulatorDriver {}

impl<T: SimulatorDrive + DifftestRefSingleCycleDrive + DifftestRefPipelineDrive> SimulatorDriver for T {}

pub struct EmuWrapper<V> {
    emu: Emu,
    _marker: std::marker::PhantomData<V>,
}

impl<V: SimulatorDrive> SimulatorItem for EmuWrapper<V> {
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

impl<V: DifftestRefSingleCycleDrive> DifftestRefSingleCycleApi for EmuWrapper<V> {
    fn instruction_compelete(&mut self) -> ProcessResult<()> {
        V::instruction_compelete(&mut self.emu)
    }
}

impl<V: DifftestRefPipelineDrive> DifftestRefPipelineApi for EmuWrapper<V> {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        V::step_cycle(&mut self.emu)
    }

    fn instruction_fetch_enable(&mut self) {
        V::instruction_fetch_enable(&mut self.emu)
    }

    fn load_store_enable(&mut self) {
        V::load_store_enable(&mut self.emu)
    }
}

impl<V> EmuWrapper<V> {
    pub fn new(option: &OptionParser, states: States, callback: SimulatorCallback) -> Self {
        Self {
            emu: Emu::new(option, states, callback),
            _marker: std::marker::PhantomData,
        }
    }
}
