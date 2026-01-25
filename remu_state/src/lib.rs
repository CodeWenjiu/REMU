use std::marker::PhantomData;

use remu_types::Rv32Isa;

use crate::{bus::Bus, reg::RiscvReg};

remu_macro::mod_pub!(reg, bus);
remu_macro::mod_flat!(option, command);

/// State template
pub struct State<I: Rv32Isa> {
    pub bus: Bus,
    pub reg: RiscvReg,
    tracer: remu_types::TracerDyn,
    _marker: PhantomData<I>,
}

impl<I: Rv32Isa> State<I> {
    pub fn new(opt: StateOption, tracer: remu_types::TracerDyn) -> Self {
        Self {
            bus: Bus::new(opt.bus, tracer.clone()),
            reg: RiscvReg::new(opt.reg, tracer.clone()),
            tracer,
            _marker: PhantomData,
        }
    }

    pub fn execute(&mut self, subcmd: &StateCmd) {
        match subcmd {
            StateCmd::Bus { subcmd } => self.bus.execute(subcmd),
            StateCmd::Reg { subcmd } => self.reg.execute(subcmd),
            StateCmd::MemMap => {
                let map = self.bus.mem_map();
                self.tracer.borrow().mem_show_map(map);
            }
        }
    }
}
