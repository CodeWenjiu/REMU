use remu_state::{State, StateCmd};
use remu_types::{DifftestMismatchItem, TracerDyn};

use crate::error::SimulatorError;
use crate::{FuncCmd, SimulatorOption};

pub trait SimulatorTrait<P: remu_state::StatePolicy, const IS_DUT: bool = true> {
    const ENABLE: bool = true;

    fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self;

    fn state(&self) -> &State<P>;

    #[inline(always)]
    fn step_once(&mut self) -> Result<(), SimulatorError> {
        let _ = self;
        Ok(())
    }

    #[inline(always)]
    fn sync_from(&mut self, dut: &State<P>) {
        let _ = (self, dut);
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
    fn func_exec(&mut self, _subcmd: &FuncCmd) {
        let _ = self;
    }

    #[inline(always)]
    fn state_exec(&mut self, _subcmd: &StateCmd) -> Result<(), SimulatorError> {
        let _ = self;
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
}
