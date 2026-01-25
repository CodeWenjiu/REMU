remu_macro::mod_pub!(riscv);
remu_macro::mod_flat!(option, command, func);

use remu_types::TracerDyn;
use target_lexicon::Architecture;

use crate::riscv::new_simulator_riscv;

pub trait Simulator {
    fn exec(&mut self, command: &Command);
}

pub fn new_simulator(option: SimulatorOption, tracer: TracerDyn) -> Box<dyn Simulator> {
    match option.isa.0 {
        Architecture::Riscv32(isa) => new_simulator_riscv(option, isa, tracer),
        _ => unreachable!(),
    }
}
