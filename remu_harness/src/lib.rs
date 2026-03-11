remu_macro::mod_flat!(error, func, option, policy, run_state,);

pub use error::HarnessError;
pub use option::HarnessOption;
pub use policy::HarnessPolicy;
pub use run_state::RunState;

pub use remu_simulator::{
    DifftestMismatchList, FuncCmd, SimulatorCore, SimulatorDut, SimulatorError,
    SimulatorInnerError, SimulatorPolicy, SimulatorPolicyOf, SimulatorRef, TraceCmd,
};
use remu_types::{AllUsize, DifftestMismatchItem, RegGroup, TraceKind};
pub use remu_simulator_remu::SimulatorRemu;
pub use remu_simulator_nzea::SimulatorNzea;
pub use remu_simulator_spike::SimulatorSpike;
pub use remu_state::StateCmd;
pub use remu_state::bus::ObserverEvent;
pub use remu_types::ExitCode;

/// Outcome of a run (e.g. run_steps). Propagated to debugger and CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    /// Run stopped without program exit (limit reached or already idle).
    Done,
    /// Program requested exit (e.g. ecall).
    ProgramExit(ExitCode),
}

impl RunOutcome {
    /// Prefer ProgramExit over Done when merging outcomes from multiple commands.
    #[inline(always)]
    pub fn or_else(self, other: RunOutcome) -> RunOutcome {
        match self {
            RunOutcome::ProgramExit(_) => self,
            RunOutcome::Done => other,
        }
    }
}

use remu_types::TracerDyn;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Harness<D, R> {
    dut_model: D,
    ref_model: R,
    func: func::Func,
    interrupt: Arc<AtomicBool>,
    run_state: RunState,
}

impl<D, R> Harness<D, R>
where
    D: SimulatorDut,
    R: SimulatorRef<D::Policy>,
{
    pub fn new(opt: HarnessOption, tracer: TracerDyn, interrupt: Arc<AtomicBool>) -> Self {
        let mut dut_model = D::new(opt.sim.clone(), tracer.clone(), Arc::clone(&interrupt));
        let mut ref_model = R::new(opt.sim, tracer, Arc::clone(&interrupt));
        dut_model.init();
        ref_model.init();
        Self {
            dut_model,
            ref_model,
            func: func::Func::new(),
            interrupt,
            run_state: RunState::Idle,
        }
    }

    #[inline(always)]
    pub fn run_state(&self) -> RunState {
        self.run_state
    }

    #[inline(always)]
    fn step_once<const TRACE: u64>(&mut self) -> Result<(), SimulatorError> {
        self.dut_model
            .step_once::<TRACE>()
            .map_err(SimulatorError::Dut)?;
        if R::ENABLE {
            let events = self.dut_model.take_observer_events();
            let mut need_sync = false;
            let mut mem_writes: Vec<(usize, Box<[u8]>)> = Vec::new();
            for e in &events {
                match e {
                    ObserverEvent::MmioAccess => need_sync = true,
                    ObserverEvent::MemoryWrite(addr, data) => {
                        mem_writes.push((*addr, data.clone()));
                    }
                }
            }
            if need_sync {
                self.ref_model.sync_regs_from(&self.dut_model.state().reg);
                return Ok(());
            }
            self.ref_model
                .step_once::<0>()
                .map_err(SimulatorError::Ref)?;
            let mut diff = self.ref_model.regs_diff(&self.dut_model.state().reg);
            for (addr, dut_data) in &mem_writes {
                if let Some(ref_bytes) =
                    self.ref_model.mem_compare(*addr, dut_data.as_ref())
                {
                    diff.push(DifftestMismatchItem {
                        group: RegGroup::Mem,
                        name: format!("0x{:08x}:{}", addr, dut_data.len()),
                        ref_val: AllUsize::Bytes(ref_bytes),
                        dut_val: AllUsize::Bytes(dut_data.clone()),
                    });
                }
            }
            if !diff.is_empty() {
                return Err(SimulatorError::Difftest(DifftestMismatchList(diff)));
            }
        }
        Ok(())
    }

    pub fn func_exec(&mut self, subcmd: &FuncCmd) {
        self.func.execute(subcmd);
        if let FuncCmd::Trace { subcmd: trace } = subcmd {
            let (kind, enabled) = match trace {
                TraceCmd::Instruction { enable } => (TraceKind::Instruction, *enable),
                TraceCmd::WaveForm { enable } => (TraceKind::Wavetrace, *enable),
            };
            self.dut_model.on_trace_change(kind, enabled);
        }
    }

    pub fn state_exec(&mut self, subcmd: &StateCmd) -> Result<(), HarnessError> {
        self.dut_model
            .state_exec(subcmd)
            .map_err(SimulatorError::Dut)
            .map_err(HarnessError::from)
    }

    pub fn ref_state_exec(&mut self, subcmd: &StateCmd) -> Result<(), HarnessError> {
        self.ref_model
            .state_exec(subcmd)
            .map_err(SimulatorError::Ref)
            .map_err(HarnessError::from)
    }

    /// Set a breakpoint at the given address on the DUT.
    /// Fails if the address is not 4-byte aligned or not mapped to memory.
    #[inline(always)]
    pub fn set_breakpoint(&mut self, addr: u32) -> Result<(), HarnessError> {
        self.dut_model
            .set_breakpoint(addr)
            .map_err(SimulatorError::Dut)
            .map_err(HarnessError::from)
    }

    /// Delete a breakpoint at the given address on the DUT.
    /// Fails if the breakpoint does not exist.
    #[inline(always)]
    pub fn del_breakpoint(&mut self, addr: u32) -> Result<(), HarnessError> {
        self.dut_model
            .del_breakpoint(addr)
            .map_err(SimulatorError::Dut)
            .map_err(HarnessError::from)
    }

    /// Print all breakpoints (via DUT tracer).
    #[inline(always)]
    pub fn print_breakpoints(&self) {
        self.dut_model.print_breakpoints();
    }

    /// Run steps in batch until limit reached, interrupt set, program exit, or error.
    /// Uses the harness's `interrupt` and `run_state`; sets `run_state` to `Exit` on program exit.
    /// Returns `Ok(RunOutcome::ProgramExit(code))` when program exited; `Ok(RunOutcome::Done)` when stopped without exit.
    /// Returns `Err(HarnessError::Interrupted)` when `interrupt` was set.
    /// Trace flags are read once and fixed for the whole run.
    pub fn run_steps(&mut self, max_steps: Option<usize>) -> Result<RunOutcome, HarnessError> {
        const BATCH: usize = 4096;
        if self.run_state == RunState::Exit {
            return Ok(RunOutcome::Done);
        }
        let trace = self.func.trace.flags.bits();
        match trace {
            0 => self.run_steps_impl::<0>(max_steps, BATCH),
            1 => self.run_steps_impl::<1>(max_steps, BATCH),
            2 => self.run_steps_impl::<2>(max_steps, BATCH),
            3 => self.run_steps_impl::<3>(max_steps, BATCH),
            _ => self.run_steps_impl::<0>(max_steps, BATCH), // Fallback for unknown trace combo
        }
    }

    #[inline(always)]
    fn run_steps_impl<const TRACE: u64>(
        &mut self,
        max_steps: Option<usize>,
        batch: usize,
    ) -> Result<RunOutcome, HarnessError> {
        let mut steps = 0usize;
        loop {
            if self.interrupt.load(Ordering::Relaxed) {
                self.interrupt.store(false, Ordering::Relaxed);
                return Err(HarnessError::Interrupted);
            }
            if max_steps.map_or(false, |limit| steps >= limit) {
                return Ok(RunOutcome::Done);
            }
            let to_run = max_steps
                .map(|limit| (limit - steps).min(batch))
                .unwrap_or(batch);
            for _ in 0..to_run {
                match self.step_once::<TRACE>() {
                    Ok(()) => steps += 1,
                    Err(SimulatorError::Dut(SimulatorInnerError::ProgramExit(exit_code))) => {
                        self.run_state = RunState::Exit;
                        return Ok(RunOutcome::ProgramExit(exit_code));
                    }
                    Err(SimulatorError::Dut(SimulatorInnerError::Interrupted))
                    | Err(SimulatorError::Ref(SimulatorInnerError::Interrupted)) => {
                        return Err(HarnessError::Interrupted);
                    }
                    Err(e) => return Err(HarnessError::from(e)),
                }
            }
        }
    }
}
