use option_parser::OptionParser;
use owo_colors::OwoColorize;
use remu_utils::ProcessResult;
use state::{reg::AnyRegfile, States};

use crate::SimulatorCallback;

use super::{AnyDifftestRef, DifftestRefFfiApi};

pub struct DifftestManager {
    pub reference: AnyDifftestRef,
    pub states_ref: States,
    pub states_dut: States,

    pub memory_watch_point: Vec<u32>,
    pub skip_count: usize,
    pub is_instruction_complete: bool,
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
            Box::new(|| {}),
        );

        let reference = AnyDifftestRef::new(option, states_ref.clone(), ref_callback);

        Self {
            reference,
            states_ref,
            states_dut,

            memory_watch_point: vec!(),
            skip_count: 0,
            is_instruction_complete: false,
        }
    }

    pub fn init(&mut self, regfile: &AnyRegfile, bin: Vec<u8>, reset_vector: u32) {
        match &mut self.reference {
            AnyDifftestRef::FFI(reference) => reference.init(regfile, bin, reset_vector),
                    
            _ => ()
        }
    }

    pub fn step_skip(&mut self) {
        self.skip_count += 1;
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
        match &mut self.reference {
            AnyDifftestRef::FFI(_) | 
            AnyDifftestRef::SingleCycle(_) => {
                    if self.is_instruction_complete {
                        self.step_single_instruction()?;
                        self.is_instruction_complete = false;
                    }
                }

            AnyDifftestRef::Pipeline(_) => {}
        }

        Ok(())
    }
}
