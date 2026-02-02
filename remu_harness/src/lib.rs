//! Harness：容纳 DUT 与 ref 的壳，权责与模拟器实现完全分离。
//!
//! 本 crate 仅负责：持有两个实现 [SimulatorTrait] 的实例（DUT 与 ref），
//! 驱动单步（`dut.step_once()` → 可选 `ref.step_once()` + 比较），以及将 func/state 命令转发给 DUT。
//! 具体的取指、译码、执行、trace 等逻辑均位于 `remu_simulator`。

mod option;

pub use option::HarnessOption;

// Re-exports：Debugger 等仅依赖 remu_harness 即可。
pub use remu_simulator::riscv::SimulatorError;
pub use remu_simulator::{
    FuncCmd, SimulatorDut, SimulatorPolicy, SimulatorPolicyOf, SimulatorRemu, SimulatorTrait,
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
}
