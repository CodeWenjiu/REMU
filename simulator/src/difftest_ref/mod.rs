use option_parser::OptionParser;
use logger::Logger;
use remu_macro::log_todo;
use remu_utils::{DifftestBuildIn, DifftestFFI, DifftestRef, ProcessResult};
use state::{reg::AnyRegfile, States};

use crate::{emu::Emu, SimulatorCallback};

use enum_dispatch::enum_dispatch;

remu_macro::mod_flat!(difftest_ffi, difftest);

#[enum_dispatch]
pub enum AnyDifftestFfiRef {
    SPIKE(Spike),
}

#[enum_dispatch(AnyDifftestFfiRef)]
pub trait DifftestRefFfiApi {
    fn step_cycle(&mut self) -> ProcessResult<()>;

    fn test_reg(&self, dut: &AnyRegfile) -> ProcessResult<()>;

    fn test_mem(&mut self, watchpoint: Vec<(u32, u32)>) -> ProcessResult<()>;

    fn set_ref(&self, _target: &AnyRegfile) {
        log_todo!();
    }

    fn set_mem(&self, _addr: u32, _data: Vec<u8>) {
        log_todo!();
    }
}

#[enum_dispatch]
pub enum AnyDifftestBuildInRef {
    EMU(Emu),
}

#[enum_dispatch(AnyDifftestBuildInRef)]
pub trait DifftestRefBuildInApi {
    fn instruction_compelete(&mut self) -> ProcessResult<()>;
}

pub enum AnyDifftestRef {
    FFI(AnyDifftestFfiRef),
    BuildIn(AnyDifftestBuildInRef),
}

impl TryFrom<(&OptionParser, States, SimulatorCallback)> for AnyDifftestRef {
    type Error = ();

    fn try_from((option, states, callback): (&OptionParser, States, SimulatorCallback)) -> Result<Self, Self::Error> {
        let r#ref = option.cli.differtest.unwrap();
        match r#ref {
            DifftestRef::BuildIn(ref r#ref) => {
                match r#ref {
                    DifftestBuildIn::EMU => Ok(AnyDifftestRef::BuildIn(AnyDifftestBuildInRef::EMU(Emu::new(option, states, callback)))),
                }
            }
            DifftestRef::FFI(ref r#ref) => {
                match r#ref {
                    DifftestFFI::SPIKE => Ok(AnyDifftestRef::FFI(AnyDifftestFfiRef::SPIKE(Spike {}))),
                }
            }
        }
    }
}
