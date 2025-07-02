use std::collections::VecDeque;

use option_parser::OptionParser;
use owo_colors::OwoColorize;
use remu_macro::log_err;
use remu_utils::{ProcessError, ProcessResult};
use state::{reg::{AnyRegfile, RegfileIo}, CheckFlags4reg, States};

use crate::{difftest_ref::{DifftestRefPipelineApi, DifftestRefSingleCycleApi}, SimulatorCallback};

use super::{AnyDifftestRef, DifftestRefFfiApi};

pub struct DifftestManager {
    pub reference: AnyDifftestRef,
    pub states_ref: States,
    pub states_dut: States,

    pub memory_watch_point: Vec<u32>,
    ls_skip_val: VecDeque<u32>,
    is_instruction_complete: bool,

    is_branch_prediction: bool,
    is_instruction_fetch: bool,
    is_load_store: bool,
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
            Box::new(|_u32| {}),
            Box::new(|| {}),
            Box::new(|| {}),
            Box::new(|| {}),
            Box::new(|| {}),
        );

        let reference = AnyDifftestRef::new(option, states_ref.clone(), ref_callback);

        Self {
            reference,
            states_ref,
            states_dut,

            memory_watch_point: vec!(),
            ls_skip_val: VecDeque::new(),
            is_instruction_complete: false,

            is_branch_prediction: false,
            is_instruction_fetch: false,
            is_load_store: false,
        }
    }

    pub fn init(&mut self, regfile: &AnyRegfile, bin: Vec<u8>, reset_vector: u32) {
        match &mut self.reference {
            AnyDifftestRef::FFI(reference) => reference.init(regfile, bin, reset_vector),
                    
            _ => ()
        }
    }

    pub fn step_skip(&mut self, val: u32) {
        self.ls_skip_val.push_back(val);
    }

    pub fn instruction_complete(&mut self) {
        self.is_instruction_complete = true;
    }

    pub fn branch_prediction(&mut self) {
        self.is_branch_prediction = true;
    }

    pub fn instruction_fetch(&mut self) {
        self.is_instruction_fetch = true;
    }

    pub fn load_store(&mut self) {
        self.is_load_store = true;
    }

    pub fn push_memory_watch_point(&mut self, addr: u32) {
        self.memory_watch_point.push(addr);
    }

    pub fn show_memory_watch_point(&self) {
        for addr in &self.memory_watch_point {
            println!("{:#010x}", addr.blue());
        }
    }

    pub fn step_cycle(&mut self) -> ProcessResult<()> {
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

            AnyDifftestRef::FFI(reference) => {
                if self.is_instruction_complete {
                    if let Some(_) = self.ls_skip_val.pop_front() {
                        reference.set_ref(&self.states_dut.regfile);
                    } else {
                        reference.step_cycle()?;
                    }
                    self.is_instruction_complete = false;
                }

                reference.test_reg(&self.states_dut.regfile)?;
                reference.test_mem(mem_diff_msg)?;
            }

            AnyDifftestRef::SingleCycle(reference) => {
                if self.is_instruction_complete {
                    if let Some(_) = self.ls_skip_val.pop_front() {
                        self.states_ref.regfile.sync_reg(&self.states_dut.regfile);
                    } else {
                        reference.instruction_compelete()?;
                    }
                    self.is_instruction_complete = false;
                }

                self.states_ref.regfile.check(&self.states_dut.regfile, CheckFlags4reg::pc.union(CheckFlags4reg::gpr).union(CheckFlags4reg::csr))?;
                self.states_ref.mmu.check(mem_diff_msg)?;
            }
            
            AnyDifftestRef::Pipeline(reference) => {
                if self.is_branch_prediction {
                    reference.branch_prediction_enable();
                    self.is_branch_prediction = false;
                }

                if self.is_instruction_fetch {
                    reference.instruction_fetch_enable();
                    self.is_instruction_fetch = false;
                }

                if self.is_load_store {
                    reference.load_store_enable();
                    self.is_load_store = false;
                }

                reference.step_cycle(self.ls_skip_val.pop_front())?;

                self.states_ref.regfile.check(&self.states_dut.regfile, CheckFlags4reg::pc.union(CheckFlags4reg::gpr).union(CheckFlags4reg::csr))?;
                self.states_ref.pipe_state.as_ref()
                    .zip(self.states_dut.pipe_state.as_ref())
                    .map(|(ref_pipe, dut_pipe)| ref_pipe.check(dut_pipe))
                    .transpose()?;
                self.states_ref.mmu.check(mem_diff_msg)?;
            }

        }

        Ok(())
    }
}
