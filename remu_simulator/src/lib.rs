remu_macro::mod_pub!(riscv);
remu_macro::mod_flat!(option, command, func);
use std::marker::PhantomData;

use crate::riscv::{SimulatorError, inst::opcode::decode};
use remu_state::State;
use remu_types::{RvIsa, TracerDyn};

/// As a template
pub struct Simulator<I: RvIsa> {
    state: State<I>,
    func: Func,
    tracer: TracerDyn,
    _marker: PhantomData<I>,
}

impl<I: RvIsa> Simulator<I> {
    pub fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self {
        Self {
            state: State::new(opt.state, tracer.clone()),
            func: Func::new(),
            tracer,
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    fn step_once(&mut self) -> Result<(), SimulatorError> {
        let pc = self.state.reg.pc;
        let inst = self.state.bus.read_32(pc as usize)?;
        let decoded = decode::<I>(inst);
        (decoded.handler)(&mut self.state, &decoded)?;
        if self.func.trace.instruction {
            self.tracer.borrow().disasm(pc as u64, inst);
        }
        Ok(())
    }

    fn step(&mut self, times: usize) {
        for _ in 0..times {
            match self.step_once() {
                Ok(()) => (),
                Err(err) => {
                    self.tracer.borrow().deal_error(Box::new(err));
                    break;
                }
            }
        }
    }

    pub fn exec(&mut self, command: &Command) {
        match command {
            Command::Continue => self.step(usize::MAX),
            Command::Func { subcmd } => self.func.execute(subcmd),
            Command::State { subcmd } => self.state.execute(subcmd),
            Command::Step { times } => self.step(*times),
        }
    }
}
