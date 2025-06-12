use option_parser::OptionParser;
use logger::Logger;
use owo_colors::OwoColorize;
use remu_macro::log_err;
use remu_utils::{ProcessError, ProcessResult};
use state::{reg::{AnyRegfile, RegfileIo}, CheckFlags4reg, States};

use crate::SimulatorCallback;

use super::{AnyDifftestRef, DifftestRefSingleCycleApi, DifftestRefFfiApi};

pub struct DifftestSingleCycleManager {
    pub reference: AnyDifftestRef,
    pub states_ref: States,
    pub states_dut: States,

    memory_watch_point: Vec<u32>,
    is_diff_skip: bool,
}

pub enum DifftestManager {
    SingleCycle(DifftestSingleCycleManager),
}

impl DifftestSingleCycleManager {
    pub fn new(
        option: &OptionParser,
        states_dut: States,
        states_ref: States,
    ) -> Self {
        // Create a minimal callback for the reference simulator, may be useful in future
        let ref_callback = SimulatorCallback::new(
            Box::new(|_: u32, _: u32, _: u32| Ok(())),
            Box::new(|| {}),
            Box::new(|| {}),
        );

        let reference = AnyDifftestRef::new(option, states_ref.clone(), ref_callback);

        Self {
            reference,
            states_ref,
            states_dut,

            memory_watch_point: vec!(),
            is_diff_skip: false,
        }
    }

    pub fn init(&mut self, regfile: &AnyRegfile, bin: Vec<u8>, reset_vector: u32) {
        match &mut self.reference {
            AnyDifftestRef::FFI(reference) => reference.init(regfile, bin, reset_vector),
                    
            _ => ()
        }
    }

    fn single_instruction_compelete(&mut self) -> ProcessResult<()> {
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

            AnyDifftestRef::SingleCycle(reference) => {
                reference.instruction_compelete()?;
                self.states_ref.regfile.check(&self.states_dut.regfile, CheckFlags4reg::pc.union(CheckFlags4reg::gpr).union(CheckFlags4reg::csr))?;
                self.states_ref.mmu.check(mem_diff_msg)?;
            }

            AnyDifftestRef::FFI(reference) => {
                reference.step_cycle()?;
                reference.test_reg(&self.states_dut.regfile)?;
                reference.test_mem(mem_diff_msg)?;
            }

            _ => unreachable!()

        }

        Ok(())
    }

    fn single_instruction_skip(&mut self) {
        self.is_diff_skip = false;
        match &mut self.reference {
            AnyDifftestRef::FFI(reference) => {
                reference.set_ref(&self.states_dut.regfile);
            }

            AnyDifftestRef::SingleCycle(_reference) => {
                self.states_ref.regfile.sync_reg(&self.states_dut.regfile);
            }

            _ => unreachable!()
        }
    }

    pub fn step_single_instruction(&mut self) -> ProcessResult<()> {
        match self.is_diff_skip {
            true => {
                self.single_instruction_skip();
                Ok(())
            }

            false => {
                self.single_instruction_compelete()
            }
        }
    }

    pub fn skip_single_instruction(&mut self) {
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
