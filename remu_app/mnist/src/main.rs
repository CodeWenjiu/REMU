#![no_std]
#![no_main]

use remu_hal::{entry, FmtWrite, Uart16550, exit_success};

#[entry]
fn main() -> ! {
    unsafe { remu_hal::pre_main_init() };
    let mut uart = Uart16550::default_base();
    let _ = writeln!(uart, "Hello World");
    let _ = writeln!(uart, "Answer: {}", 42);
    let _ = remu_hal::Write::flush(&mut uart);
    exit_success()
}
