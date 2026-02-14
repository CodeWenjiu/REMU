#![feature(error_generic_member_access)]

use std::marker::PhantomData;

use crate::{bus::Bus, reg::riscv::RiscvReg};

remu_macro::mod_pub!(reg, bus);
remu_macro::mod_flat!(option, policy, command, error);

pub struct State<P: StatePolicy> {
    pub bus: Bus<P::ISA, P::Observer>,
    pub reg: RiscvReg<P::ISA>,
    _marker: PhantomData<P>,
}

impl<P: StatePolicy> State<P> {
    pub fn new(opt: StateOption, tracer: remu_types::TracerDyn, is_dut: bool) -> Self {
        Self {
            bus: Bus::new(opt.bus, tracer.clone(), is_dut),
            reg: RiscvReg::new(opt.reg, tracer.clone()),
            _marker: PhantomData,
        }
    }

    pub fn execute(&mut self, subcmd: &StateCmd) -> Result<(), StateError> {
        match subcmd {
            StateCmd::Bus { subcmd } => self.bus.execute(subcmd)?,
            StateCmd::Reg { subcmd } => self.reg.execute(subcmd),
        }
        Ok(())
    }
}
