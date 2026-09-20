#![feature(error_generic_member_access)]

use std::marker::PhantomData;

use crate::{bus::Bus, reg::riscv::RiscvReg};

remu_macro::mod_pub!(reg, bus);
remu_macro::mod_pub!(prelude, flow);
remu_macro::mod_prv!(error);

pub use error::StateError;
pub use flow::{
    StateCmd, StateFastProfile, StateMmioProfile, StateOption, StatePolicy, StateProfile,
};

pub struct State<P: StatePolicy> {
    pub bus: Bus<P::ISA, P::Observer>,
    pub reg: RiscvReg<P::ISA>,
    _marker: PhantomData<P>,
}

impl<P: StatePolicy> State<P> {
    pub fn new(opt: StateOption, tracer: remu_types::TracerDyn, is_dut: bool) -> Self {
        let bus = Bus::new(opt.bus, tracer.clone(), is_dut);
        // A loaded firmware image owns the reset vector: start there and let it
        // hand over to the program, instead of jumping straight to `--init-pc`.
        let mut reg_opt = opt.reg;
        if let Some(entry) = bus.firmware_entry() {
            reg_opt.init_pc = entry as u32;
        }
        let mut reg = RiscvReg::new(reg_opt, tracer.clone());
        // RISC-V boot convention for a loaded firmware: `a0` = hartid (this
        // machine is single-hart), `a1` = boot-info pointer (the resolved
        // device map; see `Bus::write_boot_info`). The firmware reads the
        // device addresses from there instead of hardcoding them.
        if bus.firmware_entry().is_some() {
            use remu_isa::Xlen;
            use remu_isa::isa::RvIsa;
            use remu_isa::isa::reg::RegAccess;
            // a0 = hartid (single-hart machine => 0)
            RegAccess::raw_write(&mut reg.gpr, 10, <P::ISA as RvIsa>::XLEN::from_u64(0));
            // a1 = boot info pointer (resolved device map for the firmware)
            if let Some(info) = bus.boot_info_addr() {
                RegAccess::raw_write(
                    &mut reg.gpr,
                    11,
                    <P::ISA as RvIsa>::XLEN::from_u64(info as u64),
                );
            }
        }
        Self {
            bus,
            reg,
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
