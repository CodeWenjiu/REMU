use remu_state::{State, StateCmd, StatePolicy};
use remu_types::{DifftestMismatchItem, TracerDyn};

use crate::SimulatorOption;
use crate::error::SimulatorInnerError;
use crate::policy::SimulatorPolicy;

pub trait SimulatorCore<P: StatePolicy> {
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

pub trait SimulatorDut: crate::policy::SimulatorPolicyOf + SimulatorCore<Self::Policy> {
    #[inline(always)]
    fn set_breakpoint(&mut self, addr: u32) {
        let _ = addr;
    }
}

pub trait SimulatorRef<P: SimulatorPolicy>: SimulatorCore<P> {
    const ENABLE: bool;
}

impl<P: StatePolicy> SimulatorCore<P> for () {
    fn new(_opt: SimulatorOption, _tracer: TracerDyn) -> Self {
        ()
    }

    fn state(&self) -> &State<P> {
        unreachable!("state() must not be called when ref is () (ENABLE is false)")
    }

    fn state_mut(&mut self) -> &mut State<P> {
        unreachable!("state_mut() must not be called when ref is () (ENABLE is false)")
    }
}

impl<P: SimulatorPolicy> SimulatorRef<P> for () {
    const ENABLE: bool = false;
}
