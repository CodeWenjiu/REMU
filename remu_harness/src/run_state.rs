use remu_types::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunState {
    #[default]
    Idle,
    Exit,
}

/// Outcome of a run (e.g. run_steps). Propagated to debugger and CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    /// Run stopped without program exit (limit reached or already idle).
    Done,
    /// Program requested exit (e.g. ecall).
    ProgramExit(ExitCode),
}

impl RunOutcome {
    /// Prefer ProgramExit over Done when merging outcomes from multiple commands.
    #[inline(always)]
    pub fn or_else(self, other: RunOutcome) -> RunOutcome {
        match self {
            RunOutcome::ProgramExit(_) => self,
            RunOutcome::Done => other,
        }
    }
}
