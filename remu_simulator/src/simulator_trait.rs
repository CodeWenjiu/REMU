use remu_state::{State, StateCmd, StateError};
use remu_types::TracerDyn;

use crate::policy::{SimulatorPolicy, SimulatorPolicyOf};
use crate::riscv::SimulatorError;
use crate::riscv::inst::opcode::decode;
use crate::{Func, FuncCmd, SimulatorOption};

pub struct SimulatorRemu<P: SimulatorPolicy> {
    state: State<P>,
}

impl<P: SimulatorPolicy> SimulatorPolicyOf for SimulatorRemu<P> {
    type Policy = P;
}

impl<P: SimulatorPolicy> SimulatorTrait<P> for SimulatorRemu<P> {
    const ENABLED: bool = true;

    fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self {
        Self {
            state: State::new(opt.state.clone(), tracer),
        }
    }

    fn state(&self) -> Option<&State<P>> {
        Some(&self.state)
    }

    #[inline(always)]
    fn step_once(&mut self) -> Result<(), SimulatorError> {
        let pc = self.state.reg.pc;
        let inst = self
            .state
            .bus
            .read_32(pc as usize)
            .map_err(StateError::from)?;
        let decoded = decode::<P>(inst);
        (decoded.handler)(&mut self.state, &decoded)?;
        Ok(())
    }

    #[inline(always)]
    fn sync_from(&mut self, dut: &State<P>) {
        self.state.reg.pc = dut.reg.pc;
        self.state.reg.gpr = dut.reg.gpr;
        self.state.reg.fpr = dut.reg.fpr;
    }

    #[inline(always)]
    fn regs_match(&self, dut: &State<P>) -> bool {
        self.state.reg.pc == dut.reg.pc
            && self.state.reg.gpr == dut.reg.gpr
            && self.state.reg.fpr == dut.reg.fpr
    }
}

/// DUT 侧完整模拟器：持 state + func + tracer，单步含取指/译码/执行与可选指令 trace。
pub struct SimulatorDut<P: SimulatorPolicy> {
    state: State<P>,
    func: Func,
    tracer: TracerDyn,
}

impl<P: SimulatorPolicy> SimulatorPolicyOf for SimulatorDut<P> {
    type Policy = P;
}

impl<P: SimulatorPolicy> SimulatorTrait<P> for SimulatorDut<P> {
    const ENABLED: bool = true;

    fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self {
        Self {
            state: State::new(opt.state.clone(), tracer.clone()),
            func: Func::new(),
            tracer,
        }
    }

    fn state(&self) -> Option<&State<P>> {
        Some(&self.state)
    }

    fn step_once(&mut self) -> Result<(), SimulatorError> {
        let pc = self.state.reg.pc;
        let inst = self
            .state
            .bus
            .read_32(pc as usize)
            .map_err(StateError::from)?;
        let decoded = decode::<P>(inst);
        (decoded.handler)(&mut self.state, &decoded)?;
        if self.func.trace.instruction {
            self.tracer.borrow().disasm(pc as u64, inst);
        }
        Ok(())
    }

    fn func_exec(&mut self, subcmd: &FuncCmd) {
        self.func.execute(subcmd);
    }

    fn state_exec(&mut self, subcmd: &StateCmd) -> Result<(), SimulatorError> {
        self.state.execute(subcmd)?;
        Ok(())
    }
}

pub trait SimulatorTrait<P: remu_state::StatePolicy> {
    const ENABLED: bool = true;

    fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self;

    #[inline(always)]
    fn state(&self) -> Option<&State<P>> {
        None
    }

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
        let _ = (self, dut);
        true
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

impl<P: remu_state::StatePolicy> SimulatorTrait<P> for () {
    const ENABLED: bool = false;

    fn new(_opt: SimulatorOption, _tracer: TracerDyn) -> Self {
        ()
    }
}
