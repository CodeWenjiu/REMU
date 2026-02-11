mod option;
mod policy;

pub use option::HarnessOption;
pub use policy::HarnessPolicy;

pub use remu_simulator::{
    DifftestMismatchList, SimulatorError, SimulatorInnerError, SimulatorPolicy, SimulatorPolicyOf,
    SimulatorTrait,
};
pub use remu_simulator_remu::{FuncCmd, SimulatorRemu};
pub use remu_state::StateCmd;
pub use remu_state::bus::ObserverEvent;

pub type DutSim<P> = SimulatorRemu<P, true>;
pub type RefSim<P> = SimulatorRemu<P, false>;

use remu_types::TracerDyn;

pub struct Harness<D, R> {
    dut_model: D,
    ref_model: R,
}

impl<D, R> Harness<D, R>
where
    D: SimulatorPolicyOf + SimulatorTrait<D::Policy, true>,
    R: SimulatorTrait<D::Policy, false>,
{
    pub fn new(opt: HarnessOption, tracer: TracerDyn) -> Self {
        Self {
            dut_model: D::new(opt.sim.clone(), tracer.clone()),
            ref_model: R::new(opt.sim, tracer),
        }
    }

    #[inline(always)]
    pub fn step_once(&mut self) -> Result<(), SimulatorError> {
        self.dut_model
            .step_once()
            .map_err(SimulatorError::Dut)?;
        if R::ENABLE {
            let event = self.dut_model.state_mut().bus.take_observer_event();
            match event {
                ObserverEvent::None => {
                    self.ref_model
                        .step_once()
                        .map_err(SimulatorError::Ref)?;
                    let dut_state = self.dut_model.state();
                    let diff = self.ref_model.regs_diff(dut_state);
                    if !diff.is_empty() {
                        return Err(SimulatorError::Difftest(
                            DifftestMismatchList(diff),
                        ));
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
        self.dut_model.func_exec(subcmd);
    }

    pub fn state_exec(&mut self, subcmd: &StateCmd) -> Result<(), SimulatorError> {
        self.dut_model
            .state_exec(subcmd)
            .map_err(SimulatorError::Dut)
    }

    pub fn ref_state_exec(&mut self, subcmd: &StateCmd) -> Result<(), SimulatorError> {
        self.ref_model
            .state_exec(subcmd)
            .map_err(SimulatorError::Ref)
    }
}
