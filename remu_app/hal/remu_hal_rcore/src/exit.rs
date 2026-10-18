//! Process exit via `sys_exit` (the batch system's termination contract).

use crate::syscall;

/// Exit with success (0).
pub fn exit_success() -> ! {
    let _ = syscall::sys_exit(0);
    loop {
        core::hint::spin_loop()
    }
}

/// Exit with failure (1).
pub fn exit_failure() -> ! {
    let _ = syscall::sys_exit(1);
    loop {
        core::hint::spin_loop()
    }
}
