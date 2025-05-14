use option_parser::OptionParser;
use logger::Logger;
use owo_colors::OwoColorize;
use remu_macro::log_err;
use remu_utils::{ProcessError, ProcessResult};
use state::{reg::RegfileIo, CheckFlags4reg, States};

use crate::SimulatorCallback;

use super::{AnyDifftestRef, DifftestRefBuildInApi, DifftestRefFfiApi};

pub struct DifftestManager {
    pub reference: AnyDifftestRef,
    pub states_ref: States,
    pub states_dut: States,

    memory_watch_point: Vec<u32>,
    is_diff_skip: bool,
}

impl DifftestManager {
    pub fn new(
        option: &OptionParser,
        states_dut: States,
        states_ref: States,
    ) -> Self {
        // Create a minimal callback for the reference simulator, may be useful in future
        let ref_callback = SimulatorCallback::new(
            Box::new(|_: u32, _: u32, _: u32| Ok(())),
            Box::new(|| {}),
            Box::new(|_: u32, _: u32| {}),
            Box::new(|_: bool| {}),
        );

        let reference = AnyDifftestRef::try_from((option, states_ref.clone(), ref_callback)).unwrap();

        Self {
            reference,
            states_ref,
            states_dut,

            memory_watch_point: vec!(),
            is_diff_skip: false,
        }
    }

    pub fn step_run(&mut self) -> ProcessResult<()> {
        let mem_diff_msg = self.memory_watch_point.iter()
        .map(|addr| {
            let dut_data = log_err!(
                self.states_dut.mmu.read(*addr, state::mmu::Mask::Word),
                ProcessError::Recoverable
            )?.1;
            Ok((*addr, dut_data))
        })
        .collect::<ProcessResult<Vec<_>>>()?;

        match &mut self.reference {

            AnyDifftestRef::BuildIn(reference) => {
                reference.instruction_compelete()?;
                self.states_ref.regfile.check(&self.states_dut.regfile, CheckFlags4reg::gpr.union(CheckFlags4reg::pc))?;
                self.states_ref.mmu.check(mem_diff_msg)?;
            }

            AnyDifftestRef::FFI(reference) => {
                reference.step_cycle()?;
                reference.test_reg(&self.states_dut.regfile)?;
                reference.test_mem(mem_diff_msg)?;
            }

        }

        Ok(())
    }

    pub fn step_skip(&mut self) {
        self.is_diff_skip = false;
        match &mut self.reference {
            AnyDifftestRef::BuildIn(_reference) => {
                self.states_ref.regfile.set_reg(&self.states_dut.regfile);
            }

            AnyDifftestRef::FFI(reference) => {
                reference.set_ref(&self.states_dut.regfile);
            }
        }
    }

    pub fn step(&mut self) -> ProcessResult<()> {
        match self.is_diff_skip {
            true => {
                self.step_skip();
                Ok(())
            }

            false => {
                self.step_run()
            }
        }
    }

    pub fn skip(&mut self) {
        self.is_diff_skip = true;
    }

    pub fn push_memory_watch_point(&mut self, addr: u32) {
        self.memory_watch_point.push(addr);
    }

    pub fn show_memory_watch_point(&self) {
        for addr in &self.memory_watch_point {
            println!("{:#010x}", addr.blue());
        }
    }
}
