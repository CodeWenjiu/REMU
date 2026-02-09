use remu_state::{State, StateCmd};
use remu_types::{DifftestMismatchItem, TracerDyn};

use crate::error::SimulatorInnerError;
use crate::{FuncCmd, SimulatorOption};

pub trait SimulatorTrait<P: remu_state::StatePolicy, const IS_DUT: bool = true> {
    const ENABLE: bool = true;

    fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self;

    fn state(&self) -> &State<P>;

    fn state_mut(&mut self) -> &mut State<P>;

    #[inline(always)]
    fn step_once(&mut self) -> Result<(), SimulatorInnerError> {
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
    fn func_exec(&mut self, subcmd: &FuncCmd) {
        let _ = (self, subcmd);
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
