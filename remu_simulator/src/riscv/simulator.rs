use std::marker::PhantomData;

use remu_state::{State, bus::BusAccess};
use remu_types::TracerDyn;
use target_lexicon::Riscv32Architecture;

use crate::{
    Simulator, SimulatorOption,
    riscv::{Isa, Rv32, SimulatorError, inst::opcode::decode},
};

/// As a template
pub(crate) struct SimulatorRiscv<I: Isa> {
    state: State,
    tracer: TracerDyn,
    _marker: PhantomData<I>,
}

impl<I: Isa> SimulatorRiscv<I> {
    pub(crate) fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self {
        Self {
            state: State::new(opt.state, tracer.clone()),
            tracer,
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    fn step_once(&mut self) -> Result<(), SimulatorError> {
        let inst = self.state.bus.read_32(self.state.reg.pc as usize)?;
        let decoded = decode::<I>(inst);
        (decoded.handler)(self.get_state_mut(), &decoded)?;
        Ok(())
    }
}

impl<I: Isa> Simulator for SimulatorRiscv<I> {
    fn get_state(&self) -> &State {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut State {
        &mut self.state
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
}

pub(crate) fn new_simulator_riscv(
    opt: SimulatorOption,
    isa: Riscv32Architecture,
    tracer: TracerDyn,
) -> Box<dyn Simulator> {
    match isa {
        Riscv32Architecture::Riscv32i => Box::new(SimulatorRiscv::<Rv32<false>>::new(opt, tracer)),
        Riscv32Architecture::Riscv32im => Box::new(SimulatorRiscv::<Rv32<true>>::new(opt, tracer)),
        _ => unreachable!(),
    }
}
