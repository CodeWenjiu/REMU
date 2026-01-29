remu_macro::mod_pub!(riscv);
remu_macro::mod_flat!(option, command, func);
use std::marker::PhantomData;

use crate::riscv::{SimulatorError, inst::opcode::decode};
use remu_state::{State, StateError};
use remu_types::{TracerDyn, isa::RvIsa};

/// As a template
pub struct Simulator<I: RvIsa, const DIFF_TEST: u8 = 0> {
    state: State<I>,
    func: Func,
    tracer: TracerDyn,
    _marker: PhantomData<I>,
}

impl<I: RvIsa, const DIFF_TEST: u8> Simulator<I, DIFF_TEST> {
    pub fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self {
        Self {
            state: State::new(opt.state.clone(), tracer.clone()),
            func: Func::new(),
            tracer,
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    fn step_once(&mut self) -> Result<(), SimulatorError> {
        let pc = self.state.reg.pc;
        let inst = self
            .state
            .bus
            .read_32(pc as usize)
            .map_err(StateError::from)?;
        let decoded = decode::<I, ()>(inst);
        (decoded.handler)(&mut self.state, &decoded)?;
        if self.func.trace.instruction {
            self.tracer.borrow().disasm(pc as u64, inst);
        }
        Ok(())
    }

    fn step(&mut self, times: usize) -> Result<(), SimulatorError> {
        for _ in 0..times {
            self.step_once()?;
        }
        Ok(())
    }

    pub fn exec(&mut self, command: &Command) -> Result<(), SimulatorError> {
        match command {
            Command::Continue => self.step(usize::MAX)?,
            Command::Step { times } => self.step(*times)?,

            Command::Func { subcmd } => self.func.execute(subcmd),
            Command::State { subcmd } => self.state.execute(subcmd)?,
        }
        Ok(())
    }
}
