use std::path::PathBuf;

use option_parser::OptionParser;
use remu_utils::{ProcessResult, Simulators};
use state::{reg::{AnyRegfile, RegfileIo}, CheckFlags4reg, States};

use crate::{emu::Emu, SimulatorCallback};

use enum_dispatch::enum_dispatch;

remu_macro::mod_flat!(difftest_ffi);

pub enum DifftestRefType {
    FFI {name: PathBuf},
    BuildIn {name: Simulators},
}

#[enum_dispatch(DifftestRefEnum)]
pub trait DifftestRef {
    fn step_cycle(&mut self) -> ProcessResult<()>;

    fn test_reg(&self, dut: AnyRegfile) -> bool;
}

#[enum_dispatch]
pub enum DifftestRefEnum {
    EMU(Emu),
}

impl DifftestRef for Emu {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        self.self_step_cycle()
    }

    fn test_reg(&self,dut:AnyRegfile) -> bool {
        self.states.regfile.check(dut, CheckFlags4reg::pc.union(CheckFlags4reg::gpr)).is_ok()
    }
}

impl TryFrom<(&OptionParser, States, SimulatorCallback)> for DifftestRefEnum {
    type Error = ();

    fn try_from((option, states, callback): (&OptionParser, States, SimulatorCallback)) -> Result<Self, Self::Error> {
        let sim = option.cli.differtest.unwrap();
        match sim {
            Simulators::EMU => Ok(DifftestRefEnum::EMU(Emu::new(option, states, callback))),
            _ => Err(()),
        }
    }
}

