use option_parser::OptionParser;
use remu_macro::log_todo;
use remu_utils::{DifftestPipeline, DifftestRef, DifftestSingleCycle, ProcessResult};
use state::{reg::AnyRegfile, States};

use crate::{emu::EmuWrapper, SimulatorCallback};

use enum_dispatch::enum_dispatch;

remu_macro::mod_flat!(difftest_ffi, manager);

#[enum_dispatch]
pub enum AnyDifftestFfiRef {
    TARGET(FFI),
}

#[enum_dispatch(AnyDifftestFfiRef)]
pub trait DifftestRefFfiApi {
    fn init(&mut self, regfile: &AnyRegfile, bin: Vec<u8>, reset_vector: u32);

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
pub enum AnyDifftestSingleCycleRef {
    EMU(EmuWrapper),
}

#[enum_dispatch(AnyDifftestSingleCycleRef)]
pub trait DifftestRefSingleCycleApi {
    fn instruction_compelete(&mut self) -> ProcessResult<()>;
}

#[enum_dispatch]
pub enum AnyDifftestPipelineRef {
    EMU(EmuWrapper),
}

#[enum_dispatch(AnyDifftestPipelineRef)]
pub trait DifftestRefPipelineApi {
    fn step_cycle(&mut self, skip_val: Option<u32>) -> ProcessResult<()>;

    fn instruction_fetch_enable(&mut self);
    fn load_store_enable(&mut self);

    fn get_keys(&self) -> Vec<&'static str>;
    fn print_info(&self,key: &str);
}

pub enum AnyDifftestRef {
    FFI(AnyDifftestFfiRef),
    SingleCycle(AnyDifftestSingleCycleRef),
    Pipeline(AnyDifftestPipelineRef),
}

impl AnyDifftestRef {
    pub fn new(option: &OptionParser, states: States, callback: SimulatorCallback) -> Self {
        let r#ref = option.cli.differtest.unwrap();
        match r#ref {
            DifftestRef::SingleCycle(ref r#ref) => match r#ref {
                DifftestSingleCycle::EMU => AnyDifftestRef::SingleCycle(AnyDifftestSingleCycleRef::EMU(EmuWrapper::new_dm(option, states, callback))),
            },
            DifftestRef::Pipeline(ref r#ref) => match r#ref {
                DifftestPipeline::EMU => AnyDifftestRef::Pipeline(AnyDifftestPipelineRef::EMU(EmuWrapper::new_pl(option, states, callback))),
            },
            DifftestRef::FFI(so_path) => AnyDifftestRef::FFI(AnyDifftestFfiRef::TARGET(FFI::new(&so_path))),
        }
    }
}
