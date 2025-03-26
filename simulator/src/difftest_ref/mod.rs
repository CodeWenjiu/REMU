use option_parser::OptionParser;
use logger::Logger;
use remu_macro::log_todo;
use remu_utils::{DifftestBuildIn, DifftestFFI, DifftestRef, ProcessResult};
use state::{reg::{AnyRegfile, RegfileIo}, CheckFlags4reg, States};

use crate::{emu::Emu, SimulatorCallback};

use enum_dispatch::enum_dispatch;

remu_macro::mod_flat!(difftest_ffi);

#[enum_dispatch(AnyDifftestRef)]
pub trait DifftestRefApi {
    fn step_cycle(&mut self) -> ProcessResult<()>;

    fn test_reg(&self, dut: &AnyRegfile) -> bool;

    fn set_ref(&self, _target: &AnyRegfile) {
        log_todo!();
    }

    fn set_mem(&self, _addr: u32, _data: Vec<u8>) {
        log_todo!();
    }
}

#[enum_dispatch]
pub enum AnyDifftestRef {
    EMU(Emu),
    SPIKE(Spike),
}

impl DifftestRefApi for Emu {
    fn step_cycle(&mut self) -> ProcessResult<()> {
        self.self_step_cycle()
    }

    fn test_reg(&self, dut: &AnyRegfile) -> bool {
        self.states.regfile.check(dut, CheckFlags4reg::pc.union(CheckFlags4reg::gpr)).is_ok()
    }
}

impl TryFrom<(&OptionParser, States, SimulatorCallback)> for AnyDifftestRef {
    type Error = ();

    fn try_from((option, states, callback): (&OptionParser, States, SimulatorCallback)) -> Result<Self, Self::Error> {
        let r#ref = option.cli.differtest.unwrap();
        match r#ref {
            DifftestRef::BuildIn(ref r#ref) => {
                match r#ref {
                    DifftestBuildIn::EMU => Ok(AnyDifftestRef::EMU(Emu::new(option, states, callback))),
                }
            }
            DifftestRef::FFI(ref r#ref) => {
                match r#ref {
                    DifftestFFI::SPIKE => Ok(AnyDifftestRef::SPIKE(Spike {})),
                }
            }
        }
    }
}
