use enum_dispatch::enum_dispatch;
use option_parser::OptionParser;
use remu_utils::ProcessResult;
use state::States;

use crate::{difftest_ref::{DifftestRefPipelineApi, DifftestRefSingleCycleApi}, SimulatorCallback, SimulatorItem};

use super::Emu;

#[enum_dispatch(SimulatorKind)]
pub trait EmuSimulatorCore {
    fn step_cycle(&mut self) -> ProcessResult<()>;
    fn instruction_complete(&mut self) -> ProcessResult<()> { Ok(()) }
    fn step_cycle_with_skip(&mut self, _skip: Option<u32>) -> ProcessResult<()> { Ok(()) }
    fn instruction_fetch_enable(&mut self) {}
    fn load_store_enable(&mut self) {}
    fn times(&self) -> ProcessResult<()>;
    fn function_wave_trace(&self, _enable: bool);
}

pub struct DirectlyMap {
    emu: Emu,
}
pub struct SingleCycle {
    emu: Emu,
}
pub struct Pipeline {
    emu: Emu,
}

impl EmuSimulatorCore for DirectlyMap {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        self.emu.self_step_cycle_dm()
    }
    fn instruction_complete(&mut self) -> ProcessResult<()> {
        self.emu.self_step_cycle_dm()
    }
    fn times(&self) -> ProcessResult<()> {
        self.emu.times()
    }
    fn function_wave_trace(&self, enable: bool) {
        self.emu.function_wave_trace(enable)
    }
}

impl EmuSimulatorCore for SingleCycle {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        self.emu.self_step_cycle_singlecycle()
    }
    fn instruction_complete(&mut self) -> ProcessResult<()> {
        self.emu.self_step_cycle_singlecycle()
    }
    fn times(&self) -> ProcessResult<()> {
        self.emu.times()
    }
    fn function_wave_trace(&self, enable: bool) {
        self.emu.function_wave_trace(enable)
    }
}

impl EmuSimulatorCore for Pipeline {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        self.emu.self_step_cycle_pipeline()
    }
    fn step_cycle_with_skip(&mut self, skip: Option<u32>) -> ProcessResult<()> {
        self.emu.self_step_cycle_pipeline_without_enable(skip)
    }
    fn instruction_fetch_enable(&mut self) {
        self.emu.self_pipeline_ifena();
    }
    fn load_store_enable(&mut self) {
        self.emu.self_pipeline_lsena();
    }
    fn times(&self) -> ProcessResult<()> {
        self.emu.times()
    }
    fn function_wave_trace(&self, enable: bool) {
        self.emu.function_wave_trace(enable)
    }
}

#[enum_dispatch]
pub enum SimulatorKind {
    DirectlyMap(DirectlyMap),
    SingleCycle(SingleCycle),
    Pipeline(Pipeline),
}

impl DirectlyMap {
    pub fn new(option: &OptionParser, states: States, callback: SimulatorCallback) -> Self {
        Self { emu: Emu::new(option, states, callback) }
    }
}
impl SingleCycle {
    pub fn new(option: &OptionParser, states: States, callback: SimulatorCallback) -> Self {
        Self { emu: Emu::new(option, states, callback) }
    }
}
impl Pipeline {
    pub fn new(option: &OptionParser, states: States, callback: SimulatorCallback) -> Self {
        Self { emu: Emu::new(option, states, callback) }
    }
}

pub struct EmuWrapper {
    kind: SimulatorKind,
}

impl EmuWrapper {
    pub fn new_dm(option: &OptionParser, states: States, callback: SimulatorCallback) -> Self {
        Self { kind: SimulatorKind::DirectlyMap(DirectlyMap::new(option, states, callback)) }
    }
    pub fn new_sc(option: &OptionParser, states: States, callback: SimulatorCallback) -> Self {
        Self { kind: SimulatorKind::SingleCycle(SingleCycle::new(option, states, callback)) }
    }
    pub fn new_pl(option: &OptionParser, states: States, callback: SimulatorCallback) -> Self {
        Self { kind: SimulatorKind::Pipeline(Pipeline::new(option, states, callback)) }
    }

    pub fn step_cycle(&mut self) -> ProcessResult<()> {
        self.kind.step_cycle()
    }
    pub fn instruction_complete(&mut self) -> ProcessResult<()> {
        self.kind.instruction_complete()
    }
    pub fn step_cycle_with_skip(&mut self, skip: Option<u32>) -> ProcessResult<()> {
        self.kind.step_cycle_with_skip(skip)
    }
    pub fn instruction_fetch_enable(&mut self) {
        self.kind.instruction_fetch_enable()
    }
    pub fn load_store_enable(&mut self) {
        self.kind.load_store_enable()
    }
    pub fn times(&self) -> ProcessResult<()> {
        self.kind.times()
    }
    pub fn function_wave_trace(&self, enable: bool) {
        self.kind.function_wave_trace(enable)
    }
}

impl SimulatorItem for EmuWrapper {
    fn init(&self) -> Result<(), crate::simulator::SimulatorError> {
        Ok(())
    }
    fn step_cycle(&mut self) -> ProcessResult<()> {
        self.step_cycle()
    }
    fn times(&self) -> ProcessResult<()> {
        self.times()
    }
    fn function_wave_trace(&self, enable: bool) {
        self.function_wave_trace(enable)
    }
    fn function_nvboard(&self, _enable: bool) {
        // 如果有nvboard功能可在此实现
    }
}

impl DifftestRefSingleCycleApi for EmuWrapper {
    fn instruction_compelete(&mut self) -> ProcessResult<()> {
        self.instruction_complete()
    }
}

impl DifftestRefPipelineApi for EmuWrapper {
    fn step_cycle(&mut self, skip: Option<u32>) -> ProcessResult<()> {
        self.step_cycle_with_skip(skip)
    }
    fn instruction_fetch_enable(&mut self) {
        self.instruction_fetch_enable()
    }
    fn load_store_enable(&mut self) {
        self.load_store_enable()
    }
}
