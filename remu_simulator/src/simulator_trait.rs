use remu_state::{State, StateCmd};
use remu_types::{DifftestMismatchItem, TracerDyn};

use crate::SimulatorOption;
use crate::error::SimulatorInnerError;

pub trait SimulatorTrait<P: remu_state::StatePolicy, const IS_DUT: bool = true> {
    const ENABLE: bool = true;

    fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self;

    fn state(&self) -> &State<P>;

    fn state_mut(&mut self) -> &mut State<P>;

    #[inline(always)]
    fn step_once<const ITRACE: bool>(&mut self) -> Result<(), SimulatorInnerError> {
        let _ = self;
        Ok(())
    }

    #[inline(always)]
    fn sync_from(&mut self, dut: &State<P>) {
        let _ = (self, dut);
    }

    /// Sync only registers from DUT. Use when DUT performed MMIO (no RAM change).
    /// Default: same as sync_from. Override for ref simulators that do expensive mem sync.
    #[inline(always)]
    fn sync_regs_from(&mut self, dut: &State<P>) {
        self.sync_from(dut);
    }

    #[inline(always)]
    fn regs_match(&self, dut: &State<P>) -> bool {
        self.regs_diff(dut).is_empty()
    }

    #[inline(always)]
    fn regs_diff(&self, dut: &State<P>) -> Vec<DifftestMismatchItem> {
        let _ = (self, dut);
        vec![]
    }

    #[inline(always)]
    fn state_exec(&mut self, subcmd: &StateCmd) -> Result<(), SimulatorInnerError> {
        let _ = (self, subcmd);
        Ok(())
    }
}

impl<P: remu_state::StatePolicy> SimulatorTrait<P, false> for () {
    const ENABLE: bool = false;

    fn new(_opt: SimulatorOption, _tracer: TracerDyn) -> Self {
        ()
    }

    fn state(&self) -> &State<P> {
        unreachable!("state() must not be called when ENABLE is false (ref is ())")
    }

    fn state_mut(&mut self) -> &mut State<P> {
        unreachable!("state_mut() must not be called when ENABLE is false (ref is ())")
    }
}
