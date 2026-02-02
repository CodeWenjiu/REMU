mod option;

pub use option::HarnessOption;

// Re-exports：Debugger 等仅依赖 remu_harness 即可。
pub use remu_simulator::riscv::SimulatorError;
pub use remu_simulator::{
    FuncCmd, SimulatorPolicy, SimulatorPolicyOf, SimulatorRemu, SimulatorTrait,
};
pub use remu_state::StateCmd;

use remu_types::TracerDyn;

pub struct Harness<D, R> {
    dut_model: D,
    ref_model: R,
}

impl<D, R> Harness<D, R>
where
    D: SimulatorPolicyOf + SimulatorTrait<D::Policy>,
    R: SimulatorTrait<D::Policy>,
{
    pub fn new(opt: HarnessOption, tracer: TracerDyn) -> Self {
        Self {
            dut_model: D::new(opt.sim.clone(), tracer.clone()),
            ref_model: R::new(opt.sim, tracer),
        }
    }

    #[inline(always)]
    pub fn step_once(&mut self) -> Result<(), SimulatorError> {
        self.dut_model.step_once()?;
        if R::ENABLED {
            self.ref_model.step_once()?;
            if let Some(dut_state) = self.dut_model.state() {
                if !self.ref_model.regs_match(dut_state) {
                    return Err(SimulatorError::DifftestMismatch);
                }
            }
        }
        Ok(())
    }

    pub fn func_exec(&mut self, subcmd: &FuncCmd) {
        self.dut_model.func_exec(subcmd);
    }

    pub fn state_exec(&mut self, subcmd: &StateCmd) -> Result<(), SimulatorError> {
        self.dut_model.state_exec(subcmd)
    }

    pub fn ref_state_exec(&mut self, subcmd: &StateCmd) -> Result<(), SimulatorError> {
        self.ref_model.state_exec(subcmd)
    }
}
