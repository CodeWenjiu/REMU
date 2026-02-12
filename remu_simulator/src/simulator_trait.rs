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

    /// Run up to `n` instructions in a batch. Returns the number of instructions executed.
    /// Stops on first error. Default implementation loops `step_once()`; simulators may override
    /// with an inner loop for performance when ref/difftest does not require per-instruction sync.
    #[inline(always)]
    fn step_n(&mut self, n: usize) -> Result<usize, SimulatorInnerError> {
        let mut k = 0usize;
        while k < n {
            self.step_once()?;
            k += 1;
        }
        Ok(k)
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
