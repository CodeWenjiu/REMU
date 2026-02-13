remu_macro::mod_flat!(error, func, option, policy, run_state,);

pub use error::HarnessError;
pub use option::HarnessOption;
pub use policy::HarnessPolicy;
pub use run_state::RunState;

pub use remu_simulator::{
    DifftestMismatchList, FuncCmd, SimulatorError, SimulatorInnerError, SimulatorPolicy,
    SimulatorPolicyOf, SimulatorTrait,
};
pub use remu_simulator_remu::SimulatorRemu;
pub use remu_state::StateCmd;
pub use remu_state::bus::ObserverEvent;
pub use remu_types::ExitCode;

pub type DutSim<P> = SimulatorRemu<P, true>;
pub type RefSim<P> = SimulatorRemu<P, false>;

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
    D: SimulatorPolicyOf + SimulatorTrait<D::Policy, true>,
    R: SimulatorTrait<D::Policy, false>,
{
    pub fn new(opt: HarnessOption, tracer: TracerDyn, interrupt: Arc<AtomicBool>) -> Self {
        Self {
            dut_model: D::new(opt.sim.clone(), tracer.clone()),
            ref_model: R::new(opt.sim, tracer),
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
    fn step_once<const ITRACE: bool>(&mut self) -> Result<(), SimulatorError> {
        self.dut_model
            .step_once::<ITRACE>()
            .map_err(SimulatorError::Dut)?;
        if R::ENABLE {
            let event = self.dut_model.state_mut().bus.take_observer_event();
            match event {
                ObserverEvent::None => {
                    self.ref_model
                        .step_once::<false>()
                        .map_err(SimulatorError::Ref)?;
                    let dut_state = self.dut_model.state();
                    let diff = self.ref_model.regs_diff(dut_state);
                    if !diff.is_empty() {
                        return Err(SimulatorError::Difftest(DifftestMismatchList(diff)));
                    }
                }
                ObserverEvent::MmioiAccess => {
                    self.ref_model.sync_regs_from(self.dut_model.state());
                }
            }
        }
        Ok(())
    }

    pub fn func_exec(&mut self, subcmd: &FuncCmd) {
        self.func.execute(subcmd);
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

    /// Run steps in batch until limit reached, interrupt set, program exit, or error.
    /// Uses the harness's `interrupt` and `run_state`; sets `run_state` to `Exit` on program exit.
    /// Returns `Ok(RunOutcome::ProgramExit(code))` when program exited; `Ok(RunOutcome::Done)` when stopped without exit.
    /// Returns `Err(HarnessError::Interrupted)` when `interrupt` was set.
    /// Instruction-trace flag is read once and fixed for the whole run.
    pub fn run_steps(&mut self, max_steps: Option<usize>) -> Result<RunOutcome, HarnessError> {
        const BATCH: usize = 4096;
        if self.run_state == RunState::Exit {
            return Ok(RunOutcome::Done);
        }
        if self.func.trace.instruction {
            self.run_steps_impl::<true>(max_steps, BATCH)
        } else {
            self.run_steps_impl::<false>(max_steps, BATCH)
        }
    }

    #[inline(always)]
    fn run_steps_impl<const ITRACE: bool>(
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
                match self.step_once::<ITRACE>() {
                    Ok(()) => steps += 1,
                    Err(SimulatorError::Dut(SimulatorInnerError::ProgramExit(exit_code))) => {
                        self.run_state = RunState::Exit;
                        return Ok(RunOutcome::ProgramExit(exit_code));
                    }
                    Err(e) => return Err(HarnessError::from(e)),
                }
            }
        }
    }
}
