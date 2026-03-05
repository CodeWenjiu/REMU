use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use remu_state::bus::ObserverEvent;
use remu_state::reg::riscv::RiscvReg;
use remu_state::{State, StateCmd, StatePolicy};
use remu_types::{DifftestMismatchItem, TraceKind, TracerDyn};

use crate::SimulatorOption;
use crate::error::SimulatorInnerError;
use crate::policy::SimulatorPolicy;

pub trait SimulatorCore<P: StatePolicy> {
    fn new(opt: SimulatorOption, tracer: TracerDyn, interrupt: Arc<AtomicBool>) -> Self;

    #[inline(always)]
    fn init(&mut self) {}

    /// Callback when a trace option is toggled via func. Upper layer calls this after applying the change.
    /// Default: no-op. Simulators (e.g. nzea) may open waveform file on first Wavetrace enable.
    #[inline(always)]
    fn on_trace_change(&mut self, _kind: TraceKind, _enabled: bool) {
        let _ = (_kind, _enabled);
    }

    fn state(&self) -> &State<P>;

    fn state_mut(&mut self) -> &mut State<P>;

    #[inline(always)]
    fn step_once<const TRACE: u64>(&mut self) -> Result<(), SimulatorInnerError> {
        let _ = TRACE;
        Ok(())
    }

    /// Take and clear observer events (MMIO, memory writes) from this step.
    /// Allows simulator to apply internal logic (e.g. nzea may merge/transform for difftest).
    #[inline(always)]
    fn take_observer_events(&mut self) -> Vec<ObserverEvent> {
        vec![]
    }

    #[inline(always)]
    fn sync_regs_from(&mut self, _reg: &RiscvReg<P::ISA>) {
        // Default: no-op. Ref simulators (remu, spike) override to sync regs from DUT.
    }

    #[inline(always)]
    fn regs_match(&self, dut_reg: &RiscvReg<P::ISA>) -> bool {
        self.regs_diff(dut_reg).is_empty()
    }

    #[inline(always)]
    fn regs_diff(&self, dut_reg: &RiscvReg<P::ISA>) -> Vec<DifftestMismatchItem> {
        let _ = (self, dut_reg);
        vec![]
    }

    #[inline(always)]
    fn state_exec(&mut self, subcmd: &StateCmd) -> Result<(), SimulatorInnerError> {
        let _ = (self, subcmd);
        Ok(())
    }

    /// Compare ref memory at `addr` with `dut_data`. Returns `None` if equal, else `Some(ref_bytes)` for diff report.
    /// Ref models (e.g. Spike) that do not expose state implement this via their own memory interface.
    #[inline(always)]
    fn mem_compare(&mut self, addr: usize, dut_data: &[u8]) -> Option<Box<[u8]>> {
        let _ = (addr, dut_data);
        None
    }
}

pub trait SimulatorDut: crate::policy::SimulatorPolicyOf + SimulatorCore<Self::Policy> {
    #[inline(always)]
    fn set_breakpoint(&mut self, addr: u32) -> Result<(), SimulatorInnerError> {
        let _ = addr;
        Ok(())
    }

    #[inline(always)]
    fn del_breakpoint(&mut self, addr: u32) -> Result<(), SimulatorInnerError> {
        let _ = addr;
        Ok(())
    }

    /// Print all breakpoints via the tracer (e.g. list addresses). Default: no-op.
    #[inline(always)]
    fn print_breakpoints(&self) {
        // Default: no breakpoints to print.
    }
}

pub trait SimulatorRef<P: SimulatorPolicy>: SimulatorCore<P> {
    const ENABLE: bool;
}

impl<P: StatePolicy> SimulatorCore<P> for () {
    fn new(_opt: SimulatorOption, _tracer: TracerDyn, _interrupt: Arc<AtomicBool>) -> Self {
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
