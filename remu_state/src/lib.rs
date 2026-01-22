use crate::{bus::Bus, reg::RiscvReg};

remu_macro::mod_pub!(reg, bus);
remu_macro::mod_flat!(options, commands);

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

    pub fn execute(&mut self, subcmd: &StateCmds) {
        match subcmd {
            StateCmds::Bus { subcmd } => self.bus.execute(subcmd),
            StateCmds::Reg { subcmd } => self.reg.execute(subcmd),
        }
    }
}
