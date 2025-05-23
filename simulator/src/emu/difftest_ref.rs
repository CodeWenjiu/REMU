use crate::difftest_ref::DifftestRefBuildInApi;

use super::Emu;

impl DifftestRefBuildInApi for Emu {
    fn instruction_compelete(&mut self) -> remu_utils::ProcessResult<()> {
        self.self_step_cycle_singlecycle()?;
        Ok(())
    }
}
