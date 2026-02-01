remu_macro::mod_pub!(riscv);
remu_macro::mod_flat!(option, policy, func);

use crate::riscv::{SimulatorError, inst::opcode::decode};
use remu_state::{State, StateCmd, StateError};
use remu_types::TracerDyn;

pub struct Simulator<P: SimulatorPolicy> {
    state: State<P>,
    func: Func,
    tracer: TracerDyn,
}

impl<P: SimulatorPolicy> Simulator<P> {
    pub fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self {
        Self {
            state: State::new(opt.state.clone(), tracer.clone()),
            func: Func::new(),
            tracer,
        }
    }

    #[inline(always)]
    pub fn step_once(&mut self) -> Result<(), SimulatorError> {
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

    pub fn func_exec(&mut self, subcmd: &FuncCmd) {
        self.func.execute(subcmd)
    }

    pub fn state_exec(&mut self, subcmd: &StateCmd) -> Result<(), SimulatorError> {
        self.state.execute(subcmd)?;
        Ok(())
    }
}
