remu_macro::mod_pub!(riscv);
remu_macro::mod_flat!(option);

use remu_state::State;
use remu_types::TracerDyn;

use crate::riscv::SimulatorRiscv;

pub trait Simulator {
    fn get_state(&self) -> &State;
    fn get_state_mut(&mut self) -> &mut State;
    fn step(&mut self, times: usize);
}

pub fn new_simulator(option: SimulatorOption, tracer: TracerDyn) -> impl Simulator {
    SimulatorRiscv::new(option, tracer)
}
