#![cfg_attr(target_arch = "riscv32", no_std, no_main)]

use remu_hal::{FmtWrite, Uart16550, exit_success};

#[cfg_attr(target_arch = "riscv32", remu_hal::entry)]
fn main() -> ! {
    remu_hal::init();
    let mut uart = Uart16550::default_base();
    let _ = writeln!(uart, "Hello World");
    let _ = writeln!(uart, "Answer: {}", 42);

    exit_success()
}
