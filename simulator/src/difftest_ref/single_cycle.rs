use remu_macro::log_err;
use logger::Logger;
use remu_utils::{ProcessError, ProcessResult};
use state::{reg::RegfileIo, CheckFlags4reg};

use crate::difftest_ref::{AnyDifftestRef, DifftestManager, DifftestRefFfiApi, DifftestRefSingleCycleApi};

impl DifftestManager {
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
        if self.skip_count > 0 {
            self.single_instruction_skip();
            self.skip_count -= 1;
            Ok(())
        } else {
            self.single_instruction_compelete()
        }
    }
}