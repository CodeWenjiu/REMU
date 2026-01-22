use crate::{bus::Bus, reg::RiscvReg};

remu_macro::mod_pub!(reg, bus);
remu_macro::mod_flat!(option, command);

/// State template
pub struct State {
    pub bus: Bus,
    pub reg: RiscvReg,
}

impl State {
    pub fn new(opt: StateOption, tracer: remu_types::TracerDyn) -> Self {
        Self {
            bus: Bus::new(opt.bus, tracer.clone()),
            reg: RiscvReg::new(opt.reg, tracer),
        }
    }

    pub fn execute(&mut self, subcmd: &StateCmd) {
        match subcmd {
            StateCmd::Bus { subcmd } => self.bus.execute(subcmd),
            StateCmd::Reg { subcmd } => self.reg.execute(subcmd),
        }
    }
}
