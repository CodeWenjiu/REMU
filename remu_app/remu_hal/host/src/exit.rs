//! Process exit for host runs (embedded counterpart: the SiFive test finisher).

/// Exit the host process successfully.
pub fn exit_success() -> ! {
    std::process::exit(0);
}

/// Exit the host process with a failure code.
pub fn exit_failure() -> ! {
    std::process::exit(1);
}
