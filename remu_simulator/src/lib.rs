remu_macro::mod_pub!(riscv);
remu_macro::mod_flat!(option, func);

use remu_state::StateCmd;
use remu_types::{IsaSpec, TracerDyn};
use target_lexicon::Architecture;

use crate::riscv::new_simulator_riscv;

pub trait Simulator {
    fn state_exec(&mut self, command: &StateCmd);
    fn step(&mut self, times: usize);
    fn func(&mut self, cmd: &FuncCmd);
}

pub fn new_simulator(
    option: SimulatorOption,
    isa: IsaSpec,
    tracer: TracerDyn,
) -> Box<dyn Simulator> {
    match isa.0 {
        Architecture::Riscv32(isa) => new_simulator_riscv(option, isa, tracer),
        _ => unreachable!(),
    }
}
